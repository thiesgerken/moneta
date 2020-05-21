#!/bin/bash
mkdir -p /var/backups

if [ -x /usr/bin/moneta ]; then
  DATE=$(date +%Y-%m-%d-%H%M%S)

  sudo -u moneta bash -c "moneta export" > /var/backups/moneta-$DATE.json
  echo "Exported database to /var/backups/moneta-$DATE.json"
fi
