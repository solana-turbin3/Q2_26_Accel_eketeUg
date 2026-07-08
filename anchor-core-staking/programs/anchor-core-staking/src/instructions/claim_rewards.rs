use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface, mint_to_checked, MintToChecked},
};
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::{BaseAssetV1, BaseCollectionV1},
    types::{UpdateAuthority, Attribute, Attributes, Plugin, PluginType},
    instructions::{UpdatePluginV1CpiBuilder},
    fetch_plugin,
};
use crate::Config;
use crate::error::ErrorCode;

const SECONDS_PER_DAY: i64 = 86400;

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        has_one = owner @ ErrorCode::InvalidOwner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub asset: Account<'info, BaseAssetV1>,
    #[account(
        mut,
        has_one = update_authority @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub collection: Account<'info, BaseCollectionV1>,
    /// CHECK: This account data is not used, we only verify the address
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"rewards_mint", config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is the MPL Core program
    #[account(address = Pubkey::from(MPL_CORE_ID.to_bytes()))]
    pub mpl_core_program: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<ClaimRewards>) -> Result<()> {
    // We start by fetching the existing attributes
    let attributes_fetched: Option<Attributes> = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attrs, _)| attrs);

    // If the attributes don't exist, we return an error
    require!(attributes_fetched.is_some(), ErrorCode::AssetNotStaked);

    let attributes = attributes_fetched.unwrap();

    // Prepare the Attributes list to update based on the existing attributes
    let mut attributes_list: Vec<Attribute> = Vec::with_capacity(attributes.attribute_list.len());

    // Additional auxiliary variables
    let current_timestamp = Clock::get()?.unix_timestamp;
    let mut staked_timestamp: i64 = 0;
    let mut last_claimed_timestamp: i64 = 0;
    let mut last_claimed_found = false;

    for attribute in &attributes.attribute_list {
        if attribute.key == "staked" {
            require!(attribute.value == "true", ErrorCode::AssetNotStaked);
        } else if attribute.key == "staked_at" {
            staked_timestamp = attribute.value.parse::<i64>().map_err(|_| ErrorCode::InvalidTimestamp)?;
        } else if attribute.key == "last_claimed_at" {
            last_claimed_timestamp = attribute.value.parse::<i64>().map_err(|_| ErrorCode::InvalidTimestamp)?;
            last_claimed_found = true;
        } else {
            attributes_list.push(attribute.clone());
        }
    }

    if !last_claimed_found {
        last_claimed_timestamp = staked_timestamp;
    }

    // Calculate elapsed time (in seconds) since the last claim/stake
    let elapsed_time = current_timestamp.checked_sub(last_claimed_timestamp).ok_or(ErrorCode::InvalidTimestamp)?;
    // Elapsed time in days
    let elapsed_days = elapsed_time.checked_div(SECONDS_PER_DAY).ok_or(ErrorCode::InvalidTimestamp)?;

    // We must have at least one day elapsed to claim
    require!(elapsed_days > 0, ErrorCode::NoRewardsToClaim);

    // Calculate the reward amount
    let amount = (elapsed_days as u64)
        .checked_mul(ctx.accounts.config.rewards_bps as u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_mul(10u64.pow(ctx.accounts.rewards_mint.decimals as u32))
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_div(10000u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?;

    // Prepare signing seeds for the update authority
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds = &[
        b"update_authority",
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    // Push the updated/kept staking attributes
    // Staked & Staked_at remain the same as they were
    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "true".to_string(),
    });
    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: staked_timestamp.to_string(),
    });
    // last_claimed_at is set to the last claimed timestamp + elapsed days in seconds to preserve fractional seconds
    let new_last_claimed = last_claimed_timestamp
        .checked_add(elapsed_days.checked_mul(SECONDS_PER_DAY).ok_or(ErrorCode::InvalidTimestamp)?)
        .ok_or(ErrorCode::InvalidTimestamp)?;
    attributes_list.push(Attribute {
        key: "last_claimed_at".to_string(),
        value: new_last_claimed.to_string(),
    });

    UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
    .asset(&ctx.accounts.asset.to_account_info())
    .collection(Some(&ctx.accounts.collection.to_account_info()))
    .payer(&ctx.accounts.owner.to_account_info())
    .authority(Some(&ctx.accounts.update_authority.to_account_info()))
    .system_program(&ctx.accounts.system_program.to_account_info())
    .plugin(Plugin::Attributes(Attributes { attribute_list: attributes_list }))
    .invoke_signed(&[signer_seeds])?;

    // Prepare signer seeds for config PDA
    let config_seeds = &[
        b"config",
        collection_key.as_ref(),
        &[ctx.accounts.config.bump],
    ];
    let config_signer_seeds = &[&config_seeds[..]];

    mint_to_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintToChecked {
                mint: ctx.accounts.rewards_mint.to_account_info(),
                to: ctx.accounts.user_rewards_ata.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            config_signer_seeds,
        ),
        amount,
        ctx.accounts.rewards_mint.decimals,
    )?;

    Ok(())
}
