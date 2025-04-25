#!/bin/bash

ping_node() {
    local node=$1
    local result=$(ping -c 1 -W 1 $node | grep "1 received")
    if [[ -z $result ]]; then
        echo "$node is unreachable!"
        exit 1
    fi
}

check_yes_no() {
    while true; do
        read -p "Proceed setup with $MAXNODES node(s)? (Y/n) " yn
        if [[ $yn = "" ]]; then
            yn="Y"
        fi

        case $yn in
            [Yy]* ) break;;
            [Nn]* )
                echo "Exiting."
                exit;;
            * ) echo "Please answer y or n.";;
        esac
    done;
}

NOS_DIR="project-nos"
MAXNODES=${1:-16}
set -e

echo "-- Project-Nos Setup Script --"
echo "This script takes around 15 minutes to finish."
echo
check_yes_no

# Set MAXNODE to minus 1, since we start counting from 0.
MAXNODE=$((MAXNODES - 1))
if [[ $MAXNODE -lt 0 ]]; then
    echo "No need to setup or invalid node number, exiting."
    exit
fi

# Check connection.
for i in $(seq 0 $MAXNODE); do
    ping_node "node$i" 
done

# Check if $HOME is set.
if [[ -z $HOME ]]; then
    echo "\$HOME is not set, exiting."
    echo "Note: this envvar should be set to your home directory."
    exit 1
fi

# Install PDSH.
sudo apt update
sudo apt install -y pdsh

# Setup passwordless SSH.
ssh-keygen -t rsa -N "" -f $HOME/.ssh/id_rsa
echo -e "Host *\n    StrictHostKeyChecking no" > $HOME/.ssh/config
echo -e "Host *\n    StrictHostKeyChecking no" | sudo tee -a /root/.ssh/config

SSH_KEY=$(cat $HOME/.ssh/id_rsa.pub)
sudo pdsh -w ssh:node[0-$MAXNODE] "echo $SSH_KEY >> $HOME/.ssh/authorized_keys"

# Eliminate further warnings.
IFNAME="enp65s0f0np0"
echo
echo "--------------- CLUSTER INFINIBAND INTERFACES ---------------"
pdsh -w ssh:node[0-$MAXNODE] "ifconfig $IFNAME | grep $IFNAME"
echo "-------------------------------------------------------------"

set +e
read -p "Confirm network interfaces. (continue after 5s...)" -t 5
echo
echo "Continue."
set -e

# Setup development environment.
pdsh -w ssh:node[0-$MAXNODE] "sudo apt update"
pdsh -w ssh:node[0-$MAXNODE] "sudo apt install -y build-essential git libclang-dev autoconf libtool nasm libnuma-dev"

# Setup DOCA-OFED.
DOCA_URL="https://linux.mellanox.com/public/repo/doca/2.10.0/ubuntu22.04/x86_64/"
sudo pdsh -w ssh:node[0-$MAXNODE] "curl https://linux.mellanox.com/public/repo/doca/GPG-KEY-Mellanox.pub | gpg --dearmor > /etc/apt/trusted.gpg.d/GPG-KEY-Mellanox.pub"
sudo pdsh -w ssh:node[0-$MAXNODE] "echo \"deb [signed-by=/etc/apt/trusted.gpg.d/GPG-KEY-Mellanox.pub] $DOCA_URL ./\" > /etc/apt/sources.list.d/doca.list"
pdsh -w ssh:node[0-$MAXNODE] "sudo apt update"
pdsh -w ssh:node[0-$MAXNODE] "sudo apt install -y doca-ofed"
echo
echo "----------- CLUSTER INFINIBAND INTERFACE MAPPINGS -----------"
pdsh -w ssh:node[0-$MAXNODE] "ibdev2netdev | grep $IFNAME"
echo "-------------------------------------------------------------"

set +e
read -p "Confirm DOCA-OFED installation. (continue after 5s...)" -t 5
echo
echo "Continue."
set -e

# Setup Intel ISA-L.
pdsh -w ssh:node[0-$MAXNODE] "git clone https://github.com/intel/isa-l.git $HOME/isa-l"
pdsh -w ssh:node[0-$MAXNODE] "cd $HOME/isa-l && ./autogen.sh && ./configure && make -j && sudo make install"

# Setup Rust.
pdsh -w ssh:node[0-$MAXNODE] "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -q -y"
source ~/.cargo/env
rustup install nightly
rustup default nightly
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu

# Local software packages.
sudo apt install -y cloc htop
sudo apt install -y libdw-dev libaudit-dev libssl-dev libslang2-dev libgtk2.0-dev libperl-dev libiberty-dev
sudo apt install -y libbfd-dev libcap-dev libpython2-dev libunwind-dev libzstd-dev

# Pull repository.
git config --global user.name "Jian Gao"
git config --global user.email "icicle.flw@gmail.com"

cd $HOME
git clone https://github.com/IcicleF/Nos.git $NOS_DIR
cd $HOME/$NOS_DIR
echo $MAXNODES > scripts/MAXNODES

# Distribute traces.
./scripts/utils/distribute-traces.sh

# Build project.
cargo build
cargo +nightly build --target x86_64-unknown-linux-gnu --release

sudo apt install -y python3-pip
pip3 install -r requirements.txt

cd $HOME

# Setup NFS.
sudo apt install -y nfs-kernel-server
echo "$HOME/$NOS_DIR *(rw,sync,no_root_squash,no_subtree_check)" | sudo tee -a /etc/exports
sudo exportfs -rv

pdsh -w ssh:node[1-$MAXNODE] "mkdir -p $HOME/$NOS_DIR"
pdsh -w ssh:node[1-$MAXNODE] "sudo mount -t nfs node0:$HOME/$NOS_DIR $HOME/$NOS_DIR"

# Setup Hugepages.
NR_HUGEPAGES=1024
sudo pdsh -w ssh:node[0-$MAXNODE] "echo $NR_HUGEPAGES >> /proc/sys/vm/nr_hugepages"

# Setup zsh.
NO_INPUT=1 bash -c "$(curl --fail --show-error --silent --location https://raw.githubusercontent.com/zdharma-continuum/zinit/HEAD/scripts/install.sh)"

wget https://static.icyf.me/p10k.zsh
chmod 664 p10k.zsh
mv p10k.zsh .p10k.zsh

echo 'HISTFILE="$HOME/.zsh_history"' >> $HOME/.zshrc
echo 'HISTSIZE=100000' >> $HOME/.zshrc
echo 'SAVEHIST=100000' >> $HOME/.zshrc
echo 'setopt BANG_HIST             ' >> $HOME/.zshrc
echo 'setopt EXTENDED_HISTORY      ' >> $HOME/.zshrc
echo 'setopt INC_APPEND_HISTORY    ' >> $HOME/.zshrc
echo 'setopt HIST_EXPIRE_DUPS_FIRST' >> $HOME/.zshrc
echo 'setopt HIST_IGNORE_DUPS      ' >> $HOME/.zshrc
echo 'setopt HIST_IGNORE_ALL_DUPS  ' >> $HOME/.zshrc
echo 'setopt HIST_FIND_NO_DUPS     ' >> $HOME/.zshrc
echo 'setopt HIST_IGNORE_SPACE     ' >> $HOME/.zshrc
echo 'setopt HIST_SAVE_NO_DUPS     ' >> $HOME/.zshrc
echo 'setopt HIST_REDUCE_BLANKS    ' >> $HOME/.zshrc
echo '' >> $HOME/.zshrc
echo 'zinit load zdharma-continuum/history-search-multi-word' >> $HOME/.zshrc
echo 'zinit light zsh-users/zsh-autosuggestions' >> $HOME/.zshrc
echo 'zinit light zdharma-continuum/fast-syntax-highlighting' >> $HOME/.zshrc
echo 'zinit snippet https://gist.githubusercontent.com/hightemp/5071909/raw/' >> $HOME/.zshrc
echo 'zinit ice depth"1"' >> $HOME/.zshrc
echo 'zinit light romkatv/powerlevel10k' >> $HOME/.zshrc
echo '' >> $HOME/.zshrc
echo '[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh' >> $HOME/.zshrc
echo '' >> $HOME/.zshrc
echo 'export PDSH_RCMD_TYPE=ssh' >> $HOME/.zshrc
echo 'source "$HOME/.cargo/env"' >> $HOME/.zshrc
