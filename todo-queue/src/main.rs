use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use borsh::{BorshSerialize, BorshDeserialize};

const DB_FILE: &str = "todos.bin";

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct Todo {
    pub id: u64,
    pub description: String,
    pub created_at: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct Queue<T> {
    items: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, item: T) {
        self.items.push_back(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.items.iter()
    }
}

// Implement standard helper for loading and saving the queue
fn load_queue(file_path: &str) -> io::Result<Queue<Todo>> {
    if !Path::new(file_path).exists() {
        return Ok(Queue::new());
    }

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Deserialize using borsh
    borsh::from_slice::<Queue<Todo>>(&buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn save_queue(file_path: &str, queue: &Queue<Todo>) -> io::Result<()> {
    let encoded = borsh::to_vec(queue)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut file = File::create(file_path)?;
    file.write_all(&encoded)?;
    Ok(())
}

fn print_usage() {
    println!("Todo Queue CLI App");
    println!("Usage:");
    println!("  todo add \"<task description>\" - Add a task to the queue");
    println!("  todo list                    - List all pending tasks in FIFO order");
    println!("  todo done                    - Complete the next task in the queue");
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // The first argument is the binary name.
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = args[1].as_str();

    match command {
        "add" => {
            if args.len() < 3 {
                eprintln!("Error: Description is required for adding a task.");
                print_usage();
                std::process::exit(1);
            }
            let description = args[2].clone();

            // Load existing queue
            let mut queue = load_queue(DB_FILE)?;

            // Generate unique sequential ID based on maximum current ID
            let max_id = queue.iter().map(|t| t.id).max().unwrap_or(0);
            let id = max_id + 1;

            let created_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let todo = Todo {
                id,
                description: description.clone(),
                created_at,
            };

            queue.enqueue(todo);

            // Save queue
            save_queue(DB_FILE, &queue)?;

            println!("Added task: \"{}\" (ID: {})", description, id);
        }
        "list" => {
            let queue = load_queue(DB_FILE)?;
            if queue.is_empty() {
                println!("No tasks in the queue.");
            } else {
                println!("Pending tasks (FIFO order):");
                for todo in queue.iter() {
                    println!("  ID: {:<4} | Description: {:<30} | Created: {}", todo.id, todo.description, todo.created_at);
                }
            }
        }
        "done" => {
            let mut queue = load_queue(DB_FILE)?;
            if let Some(todo) = queue.dequeue() {
                save_queue(DB_FILE, &queue)?;
                println!("Completed task: \"{}\" (ID: {})", todo.description, todo.id);
            } else {
                println!("No pending tasks in the queue.");
            }
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", command);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_operations() {
        let mut queue = Queue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.enqueue(10);
        queue.enqueue(20);
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.peek(), Some(&10));

        assert_eq!(queue.dequeue(), Some(10));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.peek(), Some(&20));

        assert_eq!(queue.dequeue(), Some(20));
        assert!(queue.is_empty());
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn test_borsh_serialization() {
        let mut queue = Queue::new();
        queue.enqueue(Todo {
            id: 1,
            description: "Test 1".to_string(),
            created_at: 1000,
        });
        queue.enqueue(Todo {
            id: 2,
            description: "Test 2".to_string(),
            created_at: 2000,
        });

        let encoded = borsh::to_vec(&queue).expect("failed to serialize");
        let decoded: Queue<Todo> = borsh::from_slice(&encoded).expect("failed to deserialize");

        assert_eq!(queue, decoded);
    }
}
