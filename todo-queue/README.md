# Persistent Todo Queue CLI

A lightweight, FIFO-based command line Todo application in Rust. It utilizes **Borsh** serialization for fast, safe binary persistence of tasks on disk.

## Features

- **Generic Queue**: Custom-built generic queue (`Queue<T>`) implementing FIFO behavior.
- **Borsh Persistence**: Saves the state of the queue to a binary file (`todos.bin`) using Borsh.
- **Sequential IDs**: Automatically handles unique, incremental task IDs.
- **Timestamps**: Stores creation time using standard Unix epoch timestamps.

## Installation

Ensure you have Rust and Cargo installed. Clone or navigate to the directory and build the project:

```bash
cargo build --release
```

## CLI Usage

### 1. Add a Task

Add a new task to the back of the queue:

```bash
cargo run -- add "Buy groceries"
```

### 2. List Tasks

List all currently pending tasks in FIFO order (oldest first):

```bash
cargo run -- list
```

### 3. Complete Next Task

Remove the oldest task from the queue and mark it as completed:

```bash
cargo run -- done
```

## Running Tests

Run the test suite to verify the correctness of the queue and serialization logic:

```bash
cargo test
```
