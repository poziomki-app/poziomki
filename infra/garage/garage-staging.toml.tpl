metadata_dir = "/var/lib/garage/meta"
data_dir     = "/var/lib/garage/data"
db_engine    = "sqlite"

replication_factor = 1

rpc_bind_addr = "[::]:3901"
rpc_secret    = "${GARAGE_RPC_SECRET}"

[s3_api]
s3_region     = "garage"
api_bind_addr = "[::]:3900"
root_domain   = ".s3.garage-staging.localhost"

[admin]
api_bind_addr = "[::]:3903"
admin_token   = "${GARAGE_ADMIN_TOKEN}"
