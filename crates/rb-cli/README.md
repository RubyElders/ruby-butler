# rb-cli Installation and Usage

## Building and Installing

### Option 1: PowerShell Script (Recommended)
```powershell
# Install release version to C:\bin
.\scripts\install.ps1 -Release

# Install debug version to C:\bin  
.\scripts\install.ps1
```

### Option 2: Batch Script
```cmd
# Install release version to C:\bin
.\scripts\install.bat
```

### Option 3: Manual Installation
```powershell
# Build release version
cargo build --release --package rb-cli

# Copy manually to C:\bin
copy target\release\rb.exe C:\bin\rb.exe
```

## Usage

Once installed, you can use `rb` from anywhere:

```bash
# Show help with styled colors
rb --help

# List Ruby installations (searches ~/.rubies by default)
rb runtime
rb rt  # short alias

# List Ruby installations in custom directory
rb runtime --directory /path/to/rubies
rb rt -d /path/to/rubies
```

## Features

- **Styled Help**: Uses cargo/uv-style colored help menus
- **Global Installation**: Installs to `C:\bin` for system-wide access
- **Ruby Detection**: Automatically finds Ruby installations
- **Colored Output**: Pretty-printed Ruby version listings
- **Latest Detection**: Shows the newest Ruby version found

## PATH Configuration

The install scripts automatically check if `C:\bin` is in your PATH. If not, add it with:

```cmd
setx PATH "%PATH%;C:\bin"
```

Then restart your terminal to use `rb` from anywhere.
