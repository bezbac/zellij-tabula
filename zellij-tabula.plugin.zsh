export ZELLIJ_TABULA_ZSH_PLUGIN_VERSION="0.1.0"

chpwd() {
  if [[ -n $ZELLIJ ]]; then
    zellij pipe --name tabula -- "'$ZELLIJ_PANE_ID' '$PWD'"
  fi
}
