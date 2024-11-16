export ZELLIJ_TABULA_ZSH_PLUGIN_VERSION="0.2.0"

# The code below is based on this github gist:
# https://gist.github.com/laggardkernel/6cb4e1664574212b125fbfd115fe90a4#chpwd-hook-in-bash

_chpwd_hook() {
  shopt -s nullglob

  local f

  if [[ "$PREVPWD" != "$PWD" ]]; then
    zellij pipe --name tabula -- "'$ZELLIJ_PANE_ID' '$PWD'"
  fi

  # refresh last working dir record
  export PREVPWD="$PWD"
}

# add `;` after _chpwd_hook if PROMPT_COMMAND is not empty
PROMPT_COMMAND="_chpwd_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
