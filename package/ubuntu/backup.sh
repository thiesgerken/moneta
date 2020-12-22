#!/bin/bash
mkdir -p /var/backups/moneta

if [ -x /usr/bin/moneta ]; then
  DATE=$(date +%Y-%m-%d-%H%M%S)

  UF=/var/backups/moneta/$DATE.json.xz
  sudo -u moneta bash -c "moneta export" | xz -z > $UF
  chmod 777 $UF
  echo "Exported database to $UF"
fi
