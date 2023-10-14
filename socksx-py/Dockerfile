# This Dockerfile is supposed to create a socks proxy server that mimics a firewall.
# This uses the python library socksx. This is a proof of concept and not meant for production.
# This won't build on apple silicon based macs. Try on Linux or Windows or intel based macs.
# Works for python versions 3.6 to 3.10

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
 && rm -rf /var/lib/apt/lists/*

RUN pip3 install click socksx

COPY ./examples/functions.py /functions.py

EXPOSE 1080
ENTRYPOINT [ "./functions.py" ]
