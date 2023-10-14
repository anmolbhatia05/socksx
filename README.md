# SOCKS toolkit for Rust
[![License: MIT](https://img.shields.io/github/license/onnovalkering/socksx.svg)](https://github.com/onnovalkering/socksx/blob/master/LICENSE)
[![codecov](https://codecov.io/github/anmolbhatia05/socksx/graph/badge.svg?token=FG143DXU0Y)](https://codecov.io/github/anmolbhatia05/socksx)
![CI](https://github.com/anmolbhatia05/socksx/actions/workflows/ci.yml/badge.svg)   
A work-in-progress SOCKS toolkit for Rust. SOCKS5 ([rfc1928](https://tools.ietf.org/html/rfc1928)) and SOCKS6 ([draft-11](https://tools.ietf.org/html/draft-olteanu-intarea-socks-6-11)) are supported.    

## Chaining Features

For `SOCKS version 5`, chaining is not supported yet. It will be added in the future.
Hence, it works in the following way: Client -> Socks5 -> Destination

For `SOCKS version 6`, chaining is supported. It means that you can chain multiple SOCKS6 proxies together.
Apart from working like version 5, it can also be used to do this - Eg. Client -> Socks6 -> Socks6 -> Destination

There is also a Python interface to socksx, see [socksx-py readme](./socksx-py/README.md).

[Doc.rs documentation link](https://docs.rs/socksx/latest)

## Client Usage
Example client usage can be found in ./socksx/examples/client.rs. To run the example, use the following command:
```bash
cargo run --example client -- --host 172.16.238.4 --port 1080 --dest_host 172.16.238.5 --dest_port 12345 --src_port 12346
```
Note: The ip addresses are just examples, you should use your own ip addresses. I created a docker network and assigned
ip addresses to the containers.

## Server Usage
### Building the binary
To build the binary, run the following command:
```bash
cargo build --release
```

To run the binary, run the following command:
```bash
./target/release/socksx --host 0.0.0.0 --port 1080 --protocol socks5
```

If you want to using the chaining feature, you can run the following command:
```bash
./target/release/socksx --host 0.0.0.0 --port 1080 --protocol socks6 --chain socks6://145.10.0.1:1080
```

### Docker Image Build

To build the Docker image for the proxy service, use the following command:

(The Dockerfile is located at the root of the repository)
```bash
docker build -t proxy:latest -f Dockerfile .
```

Create a Docker network named `net` with a specified subnet.

```bash
docker network create --subnet=172.16.238.0/24 net
```

To run the Docker container, use the following command:

```bash
docker run --network=net --ip=172.16.238.2 -p 1080:1080 --name proxy proxy:latest --host 0.0.0.0 --port 1080
```

Make sure to run these commands in the correct sequence: build the image, create the network, and then run the container.

### Docker Compose
Check out the `docker-compose-proxy.yml` or `docker-compose-extensive.yml` file at the root of the repository for an example of how to use the proxy service with Docker Compose.



## TODO
- [ ] make socksx work for macOS
- [ ] support chaining in socks 5
- [ ] update release.yml worklow and badge 
- [ ] add badge for crates link 
- [ ] add badge for docs.rs and documentation link