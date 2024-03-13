#!/bin/bash

# Initialize OCI_ENGINE variable
OCI_ENGINE=""

# First, check for Podman availability
if command -v podman &> /dev/null; then
  OCI_ENGINE="podman"
else
  # If Podman is not available, check for Docker
  if command -v docker &> /dev/null; then
    # Check if Docker is running in rootless mode
    if docker info --format '{{.SecurityOptions}}' 2>/dev/null | grep -q 'rootless'; then
      OCI_ENGINE="docker"
    else
      # If not rootless, assume rootful
      OCI_ENGINE="sudo docker"
    fi
  fi
fi

if [[ -z $OCI_ENGINE ]]; then
  echo "None of: Podman, Docker rootless, or Docker rootful is available. Please have a working OCI Engine installed on your system"
  exit 1
else
  #cd $(realpath "$(dirname "${BASH_SOURCE[0]}")") # Adapted from https://stackoverflow.com/questions/59895/how-do-i-get-the-directory-where-a-bash-script-is-located-from-within-the-script
  $OCI_ENGINE build
fi

