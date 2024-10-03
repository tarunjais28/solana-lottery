def red(s):
    bright_red = "\033[0;91m"
    reset_colour = "\033[0m"
    return bright_red + s + reset_colour


def green(s):
    bright_green = "\033[0;92m"
    reset_colour = "\033[0m"
    return bright_green + s + reset_colour


def bold(s):
    bold = "\033[1m"
    reset_colour = "\033[0m"
    return bold + s + reset_colour
