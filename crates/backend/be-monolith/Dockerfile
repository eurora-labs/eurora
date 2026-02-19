FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false eurora
COPY target/release/be-monolith /usr/local/bin/be-monolith
COPY --chown=eurora:eurora config/authz /etc/eurora/authz
RUN chmod 755 /usr/local/bin/be-monolith
USER eurora
ENV AUTHZ_MODEL_PATH=/etc/eurora/authz/model.conf
ENV AUTHZ_POLICY_PATH=/etc/eurora/authz/policy.csv
EXPOSE 8000 3000
# HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD timeout 3s bash -c "</dev/tcp/localhost/8000 && </dev/tcp/localhost/3000" || exit 1
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD timeout 3s bash -c "</dev/tcp/localhost/8000" || exit 1
CMD ["/usr/local/bin/be-monolith"]
