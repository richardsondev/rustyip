#!/bin/bash

# Create certificates directory
mkdir -p certs

# Generate CA private key
openssl genrsa -out certs/ca-key.pem 4096

# Generate CA certificate
openssl req -new -x509 -days 1 -key certs/ca-key.pem -sha256 -out certs/ca-cert.pem -subj "/C=US/ST=CA/L=Test/O=RustyIP Test CA/CN=Test CA"

# Generate server private key
openssl genrsa -out certs/server-key.pem 4096

# Generate server certificate signing request
openssl req -subj "/C=US/ST=CA/L=Test/O=RustyIP Test/CN=localhost" -new -key certs/server-key.pem -out certs/server.csr

# Create extensions file for server certificate
cat > certs/server-extfile.cnf << EOF
subjectAltName = DNS:localhost,IP:127.0.0.1
extendedKeyUsage = serverAuth
EOF

# Generate server certificate signed by CA
openssl x509 -req -days 1 -in certs/server.csr -CA certs/ca-cert.pem -CAkey certs/ca-key.pem -out certs/server-cert.pem -extfile certs/server-extfile.cnf -CAcreateserial

# Clean up CSR and extension files
rm certs/server.csr certs/server-extfile.cnf

# Add CA certificate to system trust store
cp certs/ca-cert.pem /usr/local/share/ca-certificates/rustyip-test-ca.crt
update-ca-certificates

echo "✅ Certificates generated successfully"
echo "CA Certificate: certs/ca-cert.pem"
echo "Server Certificate: certs/server-cert.pem"
echo "Server Private Key: certs/server-key.pem"
