#!/bin/bash

check_yes_no() {
    while true; do
        read -p "Modify these constants? (y/N) " yn
        if [[ $yn = "" ]]; then
            yn="N"
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

SCRIPT_DIR=$(dirname $(readlink -f "$0"))
NIC_CONF_FILE="$SCRIPT_DIR/../../nos/src/ctrl/mod.rs"

# Read current constant values
current_nic_name=$(grep -oP 'pub const NIC_NAME: &str = "\K[^"]+' $NIC_CONF_FILE)
current_nic_phyport=$(grep -oP 'pub const NIC_PHYPORT: u8 = \K\d+' $NIC_CONF_FILE)

# Display current values
echo "Using NIC:           $current_nic_name"
echo "Using physical port: $current_nic_phyport"

echo
check_yes_no
echo

# Prompt user for input
read -p "Enter the new RDMA NIC name: " new_nic_name
read -p "Enter the new physical port of the NIC above: " new_nic_phyport

# Update the Rust file
sed -i "s|pub const NIC_NAME: \\&str = \".*\";|pub const NIC_NAME: \\&str = \"$new_nic_name\";|" $NIC_CONF_FILE
sed -i "s|pub const NIC_PHYPORT: u8 = .*;|pub const NIC_PHYPORT: u8 = $new_nic_phyport;|" $NIC_CONF_FILE

echo
echo "Constants updated."
