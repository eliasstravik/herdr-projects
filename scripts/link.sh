#!/bin/sh
# The open action's pane: Shepherdr's link, until Enter closes it.
printf '\n%s\n\nPress Enter to close.\n' "$SHEPHERDR_LINK_TEXT" | sed 's/^./  &/'
read -r _
