#!/usr/bin/env zsh

chpwd() {
  if [[ -n $ZELLIJ ]]; then
    zellij pipe --name tabula -- "$ZELLIJ_PANE_ID $PWD"
  fi
}
