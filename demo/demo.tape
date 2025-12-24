# Sage Agent Demo Recording
#
# This is a VHS tape file for recording terminal demos
# Install VHS: https://github.com/charmbracelet/vhs
#
# Usage:
#   vhs demo/demo.tape
#
# Output: demo/demo.gif

# Configuration
Output demo/demo.gif
Set FontFamily "JetBrains Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set Theme "Dracula"
Set Padding 30
Set Framerate 30
Set TypingSpeed 50ms

# Initial pause
Sleep 500ms

# Show the prompt
Type "# Sage - Blazing fast code agent in Rust"
Enter
Sleep 300ms

Type "# Let's see how fast it starts..."
Enter
Sleep 500ms

# Show version (demonstrates fast startup)
Type "time sage --version"
Enter
Sleep 1500ms

# Clear and show main demo
Type "clear"
Enter
Sleep 300ms

# Interactive demo
Type "sage interactive"
Enter
Sleep 1000ms

# Type a coding task
Type "Create a Python script that fetches the top 5 GitHub trending repos and displays them in a nice table"
Sleep 500ms
Enter

# Wait for response
Sleep 8000ms

# Show the result
Type "/cost"
Enter
Sleep 2000ms

# Exit
Type "exit"
Enter
Sleep 500ms

# Final message
Type "# 10x faster startup than Node.js alternatives"
Enter
Sleep 300ms
Type "# github.com/majiayu000/sage"
Enter
Sleep 2000ms
