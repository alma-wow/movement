#!/bin/bash -e

systemctl stop  movement-full-follower.service
export DOT_MOVEMENT_PATH=$HOME/.movement
export CONTAINER_REV="a5387b1"
export AWS_DEFAULT_REGION=us-west-1
export AWS_REGION=us-west-1
export MAPTOS_CHAIN_ID=250
export AWS_ACCESS_KEY_ID="<access key>"
export AWS_SECRET_ACCESS_KEY="<secret key>"
export SYNC_PATTERN="{maptos,maptos-storage,suzuka-da-db}/**"
export SYNC_BUCKET="fork-mtnet-l-sync-bucket-sync"

/usr/bin/docker compose --env-file movement/.env -f ./movement/docker/compose/movement-full-node/snapshot/docker-compose.restore.yml up --force-recreate
