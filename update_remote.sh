#!/bin/bash
# Stop the service
sudo systemctl stop volt

# Go to project directory
cd volt-core

# Get latest code
git pull origin main

# Delete the database
rm -rf volt.db

# Rebuild the project
cargo build --release

# Restart the service
sudo systemctl restart volt
