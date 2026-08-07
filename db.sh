#!/bin/bash
set -a
source .env
set +a

docker run -d \
        --name postgres-db \
        -e POSTGRES_DB=$DATABASE \
        -e POSTGRES_USER=$USER_DB \
        -e POSTGRES_PASSWORD=$PASS_DB \
        -p $PORT_DB:5432 \
        -v pgdata:/var/lib/postgresql/data \
        -v /tmp/rust-app.socket:/var/run/postgresql \
        -v ./init.sql:/docker-entrypoint-initdb.d/init.sql\
        postgres:16