FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false eurora
COPY target/release/be-monolith /usr/local/bin/be-monolith
COPY config/authz /etc/eurora/authz
RUN chown -R eurora:eurora /usr/local/bin/be-monolith /etc/eurora
USER eurora
ENV AUTHZ_MODEL_PATH=/etc/eurora/authz/model.conf
ENV AUTHZ_POLICY_PATH=/etc/eurora/authz/policy.csv
EXPOSE 50051 3000
ENV MONOLITH_ADDR=[::]:50051
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD timeout 3s bash -c "</dev/tcp/localhost/50051 && </dev/tcp/localhost/3000" || exit 1
CMD ["/usr/local/bin/be-monolith"]
