# phpz Design Document

## Goal
phpz is a simple PHP version manager for **Linux**, **Windows**, and **macOS**, inspired by tools like `nvm`.  
It focuses on **CLI PHP only**, without interfering with system-wide installations (Apache, Nginx, IIS, etc).

The unique goal is not only to manage multiple PHP versions but also to simplify configuration management (`php.ini`) across platforms.

---

## Directory Structure

- **Linux/macOS**
  ```
  ~/.phpz/
    versions/
      8.2.12/
        bin/
        php.ini
      8.3.9/
        bin/
        php.ini
    current/   -> symlink to active version
  ```

- **Windows**
  ```
  %USERPROFILE%\.phpz\
    versions\
      8.2.12\
        php.exe
        php.ini
      8.3.9\
        php.exe
        php.ini
    shims\    # lightweight executables/batch files redirecting to "current"
    current\  # copy or link to active version
  ```

---

## Version Management

- `phpz install <version>` -> Download and extract PHP build into `versions/<version>/`.
- `phpz use <version>` -> Update `current` symlink (Linux/macOS) or regenerate shims (Windows).
- `phpz list` -> List installed versions.
- `phpz current` -> Show active version.
- `phpz reset` -> Clean PATH from external PHP installations, ensuring only phpz is active.

---

## Environment Strategy

- **Linux/macOS**: PATH points to `~/.phpz/current/bin`.
- **Windows**: PATH points to `%USERPROFILE%\.phpz\shims`.

---

## php.ini Management

Managing PHP configuration is part of phpz's vision:

1. **Centralized Config**  
   Each installed version comes with its own `php.ini` under `versions/<version>/`.  
   Users can edit it, but phpz provides helpers to simplify this.

2. **Extensions Management**  
   - Default `php.ini` has common extensions preconfigured.  
   - phpz can enable/disable extensions without requiring manual file edits.  
   - Example:  
     ```bash
     phpz ext enable intl
     phpz ext disable xdebug
     ```

   These commands will update `php.ini` accordingly.

3. **Profiles**  
   Possibility to maintain different php.ini "profiles":  
   - `dev.ini` → Xdebug enabled, error reporting verbose.  
   - `prod.ini` → Optimized memory, opcache enabled.  

   Example usage:  
   ```bash
   phpz config use dev
   phpz config use prod
   ```

4. **Cross-platform Implementation**  
   phpz will parse and modify `php.ini` using a built-in cross-platform parser (written in Rust).  
   If needed, it can fallback to a lightweight dependency (INI parser crate).

---

## Cross-platform Support

- **Linux**: tar.gz builds, symlinks.  
- **macOS**: same as Linux (works for Intel and ARM64).  
- **Windows**: zip builds, shim system, PowerShell integration.  

---

## Future Enhancements

- Support for downloading prebuilt popular extensions.  
- Provide templates for common php.ini setups (dev/prod/testing).  
- Global configuration (shared across versions).  
- Integration with Docker images for reproducibility.
