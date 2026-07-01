export ZELLIJ_TABULA_ZSH_PLUGIN_VERSION="0.4.0"

zellij() {
  command zellij "$@"
  local exit_code=$?

  # Zellij keeps the old session name in existing shells after
  # `action rename-session`, which makes later CLI calls like `pipe` hang.
  if [[ $exit_code -eq 0 && "$1" == "action" && "$2" == "rename-session" && -n "$3" ]]; then
    export ZELLIJ_SESSION_NAME="$3"
  fi

  if [[ $exit_code -eq 0 && "$1" == "ac" && "$2" == "rename-session" && -n "$3" ]]; then
    export ZELLIJ_SESSION_NAME="$3"
  fi

  return $exit_code
}

chpwd() {
  if [[ -n $ZELLIJ ]]; then
    zellij pipe --name tabula -- "'$ZELLIJ_PANE_ID' '$PWD'"
  fi
}
