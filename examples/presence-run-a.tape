# synthetic recorded tape from run e, written for this repo
interface shard-store

replicas = 3
timeout = 30
operation store = ack
operation purge = ack
cache-ttl = 300
excludes = legacy-mode
