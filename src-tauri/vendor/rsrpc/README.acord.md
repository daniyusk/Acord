# Acord rsRPC patch

This directory vendors rsRPC v0.28.0 (`062e0fd9fe34a475640a822facfee68296ba22e3`)
under its original MIT license.

Acord's patch adds incremental process start/exit handling, makes manual scans
publish their result, and caches the latest detected-process payload for clients
that connect after the initial process snapshot. The remaining connector code is
unchanged from upstream.
