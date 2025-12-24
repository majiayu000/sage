# Sage Demo Recordings

This directory contains VHS tape files for recording terminal demos.

## Prerequisites

Install VHS (terminal recorder):

```bash
# macOS
brew install vhs

# Linux
go install github.com/charmbracelet/vhs@latest

# Or download from releases
# https://github.com/charmbracelet/vhs/releases
```

## Available Demos

### Full Demo (`demo.tape`)
- Duration: ~25 seconds
- Shows: fast startup, interactive mode, task execution, cost tracking
- Best for: README hero section

```bash
vhs demo/demo.tape
```

### Quick Demo (`quick-demo.tape`)
- Duration: ~15 seconds
- Shows: startup comparison with Claude Code, quick task
- Best for: Social media, Twitter

```bash
vhs demo/quick-demo.tape
```

## Recording Tips

1. **Clean environment**: Close unnecessary apps, notifications off
2. **Warm cache**: Run the commands once before recording
3. **Resolution**: Use high DPI display for crisp GIFs
4. **Mock responses**: Consider using mock API responses for consistency

## Customization

Edit the `.tape` files to customize:

- `Set Theme`: Try "Tokyo Night", "Catppuccin", "Nord"
- `Set FontFamily`: Use a coding font
- `Set TypingSpeed`: Adjust for effect
- `Sleep`: Adjust pauses between commands

## Output

GIFs are saved to `demo/demo.gif` and `demo/quick-demo.gif`.

For README usage:
```markdown
![Sage Demo](demo/demo.gif)
```

## Converting to Other Formats

```bash
# Convert to WebM (smaller file size)
ffmpeg -i demo.gif -c:v libvpx-vp9 -crf 30 demo.webm

# Convert to MP4
ffmpeg -i demo.gif -movflags faststart -pix_fmt yuv420p demo.mp4
```
