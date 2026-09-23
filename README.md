# redissniff

Connects to Redis and runs `MONITOR`, parsing and coloring the real-time command stream by key pattern or command type. Fills the gap left by manual `redis-cli MONITOR` filtering.

## Status

**built, untested**: the TCP connection and RESP simple string parsing is built. It has not been run against a real Redis instance in this sandbox environment.

## Installation

```sh
cargo install --path .
```

## Usage

```sh
redissniff [URL]
redissniff redis://127.0.0.1:6379/
redissniff --filter "user:" redis://:password@127.0.0.1:6379/
```
