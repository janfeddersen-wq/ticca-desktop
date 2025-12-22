# FUSE Installation Guide

LibreOffice is distributed as an AppImage, which requires **FUSE 2** (Filesystem in Userspace) to run. If you see an error like `dlopen(): error loading libfuse.so.2`, you need to install FUSE 2 on your system.

> **Note**: FUSE 3 (`libfuse3`) is NOT compatible with AppImages. You need FUSE 2 (`libfuse2` / `fuse2`).

## Arch Linux / Manjaro

```bash
sudo pacman -S fuse2
```

## Debian / Ubuntu / Linux Mint

```bash
sudo apt install libfuse2
```

## Fedora / RHEL / CentOS

```bash
sudo dnf install fuse-libs
```

## openSUSE

```bash
sudo zypper install libfuse2
```

## Gentoo

```bash
sudo emerge --ask sys-fs/fuse:0
```

## NixOS

Add to your configuration:
```nix
environment.systemPackages = with pkgs; [ fuse ];
```

Or run temporarily:
```bash
nix-shell -p fuse
```

## Verify Installation

After installing, verify FUSE is available:

```bash
ls -l /usr/lib/libfuse.so.2
# or
ldconfig -p | grep fuse
```

## Troubleshooting

If FUSE is installed but you still get errors:

1. **Check library path**: The library might be in a non-standard location
   ```bash
   find /usr -name "libfuse.so*" 2>/dev/null
   ```

2. **Create symlink if needed**: Some distros install as `libfuse.so.2.x.x`
   ```bash
   sudo ln -s /usr/lib/libfuse.so.2.x.x /usr/lib/libfuse.so.2
   ```

3. **Load the fuse module**:
   ```bash
   sudo modprobe fuse
   ```

## Alternative: Extract AppImage

If you cannot install FUSE, you can extract the AppImage contents:

```bash
./LibreOffice.AppImage --appimage-extract
./squashfs-root/AppRun
```

Note: This extracts ~1GB of files and is not recommended for regular use.
