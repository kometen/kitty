# Kitty Configuration Management Tool

Kitty is a secure, Git-like configuration management tool designed for tracking, versioning, and restoring system configuration files. Unlike Git, it encrypts stored content and allows files to remain in their original locations, making it ideal for managing sensitive system configurations.

![Kitty Logo](./kitty-logo.png)

## Features

- **Secure Encryption**: All stored files are encrypted with ChaCha20-Poly1305
- **Password Protection**: Repository access requires a password
- **Dual Storage Options**: Choose between file-based or SQLite storage
- **Git-like Interface**: Familiar commands (add, diff, restore, remove)
- **File Tracking**: Track files while keeping them in their original locations
- **Privilege Handling**: Supports operations on system files requiring elevated privileges
- **Filtering and Grouping**: Organize and filter tracked files with various options

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/username/kitty.git
cd kitty

# Build the project
cargo build --release

# Optional: Install the binary to your path
cp target/release/kitty /usr/local/bin/
```

### From Cargo

```bash
cargo install kitty-config
```

## Quick Start

```bash
# Initialize a new repository
kitty init

# Or initialize with SQLite storage (recommended for larger repositories)
kitty init --sqlite

# Add files to track
kitty add /etc/nginx/nginx.conf
kitty add ~/.bashrc

# See what files are being tracked
kitty list

# See differences between stored and current files
kitty diff

# Restore a file to its stored version
kitty restore ~/.bashrc

# Stop tracking a file
kitty rm ~/.bashrc
```

## Commands

| Command | Description | Options |
|---------|-------------|---------|
| `init` | Initialize a new kitty repository | `--sqlite`: Use SQLite storage |
| `add` | Track a file in the repository | `<path>`: File to add |
| `list` | Show tracked files | `--path`: Filter by path<br>`--date`: Filter by date<br>`--group`: Group files by path |
| `diff` | Show differences between tracked and current | `<path>`: Optional file to check<br>`--only-changed`: Show only changed files<br>`--summary`: Show summary only |
| `restore` | Restore files from the repository | `<path>`: File to restore<br>`--force`: Skip confirmation<br>`--dry-run`: Show what would be done<br>`--backup`: Create backup before restoring |
| `rm` | Stop tracking a file | `<path>`: File to untrack<br>`--force`: Skip confirmation<br>`--keep-content`: Keep the content in the repository |
| `migrate-sqlite` | Migrate file content to SQLite database | `--force`: Skip confirmation |

## Storage Options

### File-Based Storage (Default)

- **Pros**: Simple structure, easy to inspect manually, files can be individually recovered
- **Cons**: Less efficient for large repositories, no transactional guarantees

### SQLite Storage

- **Pros**: Better performance for large repositories, transactional safety, single-file database
- **Cons**: Requires SQLite to be installed, slightly more complex

## How It Works

1. **Repository Structure**: Kitty creates a `.kitty` directory in your current working directory
2. **File Storage**: Original files remain in their locations; Kitty stores encrypted copies
3. **Tracking**: File paths and metadata are stored in the repository configuration
4. **Encryption**: All sensitive data is encrypted with ChaCha20-Poly1305 using your password
5. **Restoration**: Files can be restored from their encrypted versions back to their original locations

## Security

- **Encryption**: ChaCha20-Poly1305 authenticated encryption
- **Key Derivation**: PBKDF2 with 100,000 iterations
- **Storage**: All sensitive data is encrypted at rest
- **No Remote Storage**: Data remains local to your system

## Comparison with Other Tools

| Feature | Kitty | Git | Ansible | Chef |
|---------|-------|-----|---------|------|
| Encryption | ✅ | ❌ | ❌ | ❌ |
| Files remain in place | ✅ | ❌ | ✅ | ✅ |
| Learning curve | Low | Medium | High | High |
| Distributed workflow | ❌ | ✅ | ✅ | ✅ |
| Database storage | ✅ | ❌ | ❌ | ✅ |
| System-level integration | ✅ | ❌ | ✅ | ✅ |

## Use Cases

- Managing server configurations
- Tracking dotfiles across multiple machines
- Versioning sensitive configuration files
- Creating backups of critical system files before making changes
- Ensuring configuration consistency across environments

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by Git's versioning capabilities
- Built with Rust for performance and safety
- Uses SQLite for reliable database storage
