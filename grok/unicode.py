#!/usr/bin/python3

import sys
import termios
import tty
import select

# Read one character from Stdin or None if timeout
# Terminal should already be in raw mode (tty.setraw(sys.stdin.fileno()))
def rawmode_read_char(timeout=0):
    """Read a character from stdin with a timeout."""
    rlist, _, _ = select.select([sys.stdin], [], [], timeout)
    if rlist:
        return sys.stdin.read(1)
    else:
        return None

def read_caret_position():
    # read the caret position from the terminal
    print("\x1b[6n", end="", flush=True)

    # Save the current terminal settings
    old_settings = termios.tcgetattr(sys.stdin)
    try:
        # Set the terminal to raw mode
        tty.setraw(sys.stdin.fileno())

        # Read the response
        response = ""
        while True:
            char = rawmode_read_char(0.1)
            if char is None:
                return -1, -1
            if char == 'R':
                break
            response += char

        # Parse the response
        r, c = map(int, response.lstrip("\x1b[").rstrip("R").split(";"))
        return r, c
    finally:
        # Restore the terminal settings
        termios.tcsetattr(sys.stdin, termios.TCSADRAIN, old_settings)

# Get and print the caret position
row, col = read_caret_position()
print(f"Current caret position: Row {row}, Column {col}")

print("123", end="")

# Get and print the caret position
row, col = read_caret_position()
print(f"<-- Current caret position: Row {row}, Column {col}")

test_string = "Ẓ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅ"
print("Test length:", len(test_string))
print("Ẓ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅ", end="")

# Get and print the caret position
row, col = read_caret_position()
print(f"<-- Current caret position: Row {row}, Column {col}")
