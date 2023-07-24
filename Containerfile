FROM registry.fedoraproject.org/fedora:38

WORKDIR /
COPY . .
RUN sudo dnf install -y rust cargo openssl-devel pkgconfig
ENV RUST_LOG=info

CMD ["cargo", "run", "--release"]
