#!/bin/sh

chmod 755 /usr/bin/moneta
chown -R root:root /etc/moneta

chmod 644 /etc/systemd/system/moneta.service /etc/systemd/system/moneta-backup.service /etc/systemd/system/moneta-backup.timer /usr/lib/sysusers.d/moneta.conf
chown root:root /etc/systemd/system/moneta.service /etc/systemd/system/moneta-backup.service /etc/systemd/system/moneta-backup.timer /usr/lib/sysusers.d/moneta.conf

chmod 755 /var/backups/backup-moneta.sh
chown root:root /var/backups/backup-moneta.sh

systemctl daemon-reload
systemd-sysusers

systemctl enable moneta.service && systemctl start moneta.service
systemctl enable moneta-backup.timer && systemctl start moneta-backup.timer
