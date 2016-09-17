#!/bin/sh

set -x

apt-get update
apt-get install -y rtorrent git stow tmux vim
# apt-get install -y awesome awesome-extra
VAGRANT_HOME=/home/vagrant
cd $VAGRANT_HOME
if [ ! -f "$VAGRANT_HOME/dotfiles" ]; then
    git clone https://github.com/jonfk/dotfiles.git
    rm "$VAGRANT_HOME/.bashrc"
    cd dotfiles && ./init.sh
else
    cd dotfiles && git pull
fi
cd $VAGRANT_HOME
mkdir -p ./rtorrent/.rtorrent ./rtorrent/rtactive ./rtorrent/unsorted
