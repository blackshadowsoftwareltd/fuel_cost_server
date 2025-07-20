#!/bin/bash

# Docker + Nginx Setup Script
# This script creates a complete Docker and Nginx setup based on the fuel_cost_server project structure

set -e

echo "=========================================="
echo "Docker + Nginx Project Setup Script"
echo "=========================================="

# Function to prompt for user input
prompt_input() {
    local prompt="$1"
    local var_name="$2"
    local default_value="$3"
    
    if [ -n "$default_value" ]; then
        read -p "$prompt [$default_value]: " input
        eval "$var_name=\${input:-$default_value}"
    else
        read -p "$prompt: " input
        eval "$var_name=\"$input\""
    fi
}

# Collect user inputs
echo "Please provide the following information:"
echo ""

prompt_input "VPS IP address" VPS_IP ""
prompt_input "Domain name (e.g., api.example.com)" DOMAIN_NAME ""
prompt_input "Backend server port" BACKEND_PORT "8880"
prompt_input "Project name" PROJECT_NAME "my_server"
prompt_input "Rust version" RUST_VERSION "1.88.0"

echo ""
echo "SSL Configuration:"
read -p "Will you use SSL/HTTPS? (y/N): " use_ssl
USE_SSL=false
if [[ $use_ssl =~ ^[Yy]$ ]]; then
    USE_SSL=true
fi

echo ""
echo "Configuration Summary:"
echo "- VPS IP: $VPS_IP"
echo "- Domain: $DOMAIN_NAME"
echo "- Backend Port: $BACKEND_PORT"
echo "- Project Name: $PROJECT_NAME"
echo "- Rust Version: $RUST_VERSION"
echo "- SSL/HTTPS: $USE_SSL"
echo ""

read -p "Continue with setup? (y/N): " confirm
if [[ ! $confirm =~ ^[Yy]$ ]]; then
    echo "Setup cancelled."
    exit 0
fi

echo ""
echo "Creating project structure..."

# Create directory structure
mkdir -p nginx/sites-available
mkdir -p nginx/sites-enabled

echo "✓ Created directory structure"

# Create Dockerfile
cat > Dockerfile << EOF
# syntax=docker/dockerfile:1

ARG RUST_VERSION=$RUST_VERSION
ARG APP_NAME=$PROJECT_NAME

################################################################################
# Create a stage for building the application.

FROM rust:\${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache clang lld musl-dev git

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies, a cache mount to /usr/local/cargo/git/db
# for git repository dependencies, and a cache mount to /app/target/ for
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the src directory to avoid having to copy the
# source code into the container. Once built, copy the executable to an
# output directory before the cache mounted /app/target is unmounted.
RUN --mount=type=bind,source=src,target=src \\
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \\
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \\
    --mount=type=cache,target=/app/target/ \\
    --mount=type=cache,target=/usr/local/cargo/git/db \\
    --mount=type=cache,target=/usr/local/cargo/registry/ \\
    cargo build --locked --release && \\
    cp ./target/release/\$APP_NAME /bin/server

################################################################################
# Create a new stage for running the application that contains the minimal
# runtime dependencies for the application. This often uses a different base
# image from the build stage where the necessary files are copied from the build
# stage.

FROM alpine:3.18 AS final

# Create a non-privileged user that the app will run under.
ARG UID=10001
RUN adduser \\
    --disabled-password \\
    --gecos "" \\
    --home "/nonexistent" \\
    --shell "/sbin/nologin" \\
    --no-create-home \\
    --uid "\${UID}" appuser

# Create data directory and set permissions
RUN mkdir -p /data && chown appuser:appuser /data

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/
RUN chown appuser:appuser /bin/server

# Set working directory to data directory
WORKDIR /data

USER appuser

# Expose the port that the application listens on.
EXPOSE $BACKEND_PORT

# What the container should run when it is started.
CMD ["/bin/server"]
EOF

echo "✓ Created Dockerfile"

# Create docker-compose.yml
cat > compose.yaml << EOF
# Docker Compose configuration for $PROJECT_NAME

services:
  server:
    build:
      context: .
      target: final
    network_mode: host
    restart: unless-stopped

  nginx:
    image: nginx:latest
    network_mode: host
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./nginx/sites-available:/etc/nginx/sites-available
      - ./nginx/sites-enabled:/etc/nginx/sites-enabled
    depends_on:
      - server
    restart: unless-stopped
EOF

echo "✓ Created compose.yaml"

# Create main nginx.conf
cat > nginx.conf << EOF
events {
    worker_connections 1024;
}

http {
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    # Logging
    log_format main '\$remote_addr - \$remote_user [\$time_local] "\$request" '
                    '\$status \$body_bytes_sent "\$http_referer" '
                    '"\$http_user_agent" "\$http_x_forwarded_for"';

    access_log /var/log/nginx/access.log main;
    error_log /var/log/nginx/error.log;

    # Basic settings
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    types_hash_max_size 2048;

    # Rate limiting
    limit_req_zone \$binary_remote_addr zone=api:10m rate=10r/s;

    # Include sites
    include /etc/nginx/sites-enabled/*;
}
EOF

echo "✓ Created nginx.conf"

# Create HTTP-only site configuration
cat > nginx/sites-available/$DOMAIN_NAME.http-only << EOF
server {
    listen 80;
    server_name $DOMAIN_NAME;
    
    # Proxy to application
    location / {
        proxy_pass http://$VPS_IP:$BACKEND_PORT;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_connect_timeout 10s;
        proxy_send_timeout 10s;
        proxy_read_timeout 10s;
        
        # CORS headers for API
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, PUT, DELETE, OPTIONS' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;
        
        # Handle preflight requests
        if (\$request_method = 'OPTIONS') {
            add_header 'Access-Control-Allow-Origin' '*';
            add_header 'Access-Control-Allow-Methods' 'GET, POST, PUT, DELETE, OPTIONS';
            add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization';
            add_header 'Access-Control-Max-Age' 1728000;
            add_header 'Content-Type' 'text/plain; charset=utf-8';
            add_header 'Content-Length' 0;
            return 204;
        }
    }
}
EOF

echo "✓ Created HTTP-only site configuration"

# Create HTTPS site configuration
cat > nginx/sites-available/$DOMAIN_NAME << EOF
server {
    listen 80;
    server_name $DOMAIN_NAME;

    # Redirect HTTP to HTTPS
    return 301 https://\$server_name\$request_uri;
}

server {
    listen 443 ssl;
    http2 on;
    server_name $DOMAIN_NAME;

    # SSL Configuration - UPDATE THESE PATHS WITH YOUR SSL CERTIFICATES
    ssl_certificate /etc/letsencrypt/live/$DOMAIN_NAME/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN_NAME/privkey.pem;
    
    # SSL settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    
    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload";
    
    # Proxy to application
    location / {
        limit_req zone=api burst=20 nodelay;
        
        proxy_pass http://server:$BACKEND_PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_cache_bypass \$http_upgrade;
        proxy_read_timeout 86400;
        
        # CORS headers for API
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, PUT, DELETE, OPTIONS' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;
        
        # Handle preflight requests
        if (\$request_method = 'OPTIONS') {
            add_header 'Access-Control-Allow-Origin' '*';
            add_header 'Access-Control-Allow-Methods' 'GET, POST, PUT, DELETE, OPTIONS';
            add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization';
            add_header 'Access-Control-Max-Age' 1728000;
            add_header 'Content-Type' 'text/plain; charset=utf-8';
            add_header 'Content-Length' 0;
            return 204;
        }
    }
}
EOF

echo "✓ Created HTTPS site configuration"

# Enable the appropriate site configuration based on SSL choice
if [ "$USE_SSL" = true ]; then
    ln -sf ../sites-available/$DOMAIN_NAME nginx/sites-enabled/$DOMAIN_NAME
    echo "✓ Enabled HTTPS site configuration"
else
    ln -sf ../sites-available/$DOMAIN_NAME.http-only nginx/sites-enabled/$DOMAIN_NAME.http-only
    echo "✓ Enabled HTTP-only site configuration"
fi


# Create README with instructions
cat > README-DOCKER-NGINX.md << EOF
# $PROJECT_NAME - Docker + Nginx Setup

This project has been configured with Docker and Nginx based on the fuel_cost_server setup.

## Configuration

- **Domain**: $DOMAIN_NAME
- **VPS IP**: $VPS_IP
- **Backend Port**: $BACKEND_PORT
- **Project Name**: $PROJECT_NAME

## Quick Start

1. **Build and run with Docker Compose:**
   \`\`\`bash
   docker compose up --build
   \`\`\`

2. **Access your application:**
   - HTTP: http://$DOMAIN_NAME
   - Health check: http://$DOMAIN_NAME/health

## File Structure

\`\`\`
.
├── Dockerfile                     # Multi-stage Docker build
├── compose.yaml                   # Docker Compose configuration
├── nginx.conf                     # Main Nginx configuration
├── nginx/
│   ├── sites-available/
│   │   ├── $DOMAIN_NAME           # HTTPS configuration
│   │   └── $DOMAIN_NAME.http-only # HTTP-only configuration
│   └── sites-enabled/
│       └── [active configuration] # Symlink to chosen configuration
└── README-DOCKER-NGINX.md         # This file
\`\`\`

## SSL/HTTPS Setup

To enable HTTPS:

1. **Obtain SSL certificates** (e.g., using Let's Encrypt):
   \`\`\`bash
   sudo certbot certonly --standalone -d $DOMAIN_NAME
   \`\`\`

2. **Switch to HTTPS configuration:**
   \`\`\`bash
   rm nginx/sites-enabled/$DOMAIN_NAME.http-only
   ln -s /etc/nginx/sites-available/$DOMAIN_NAME nginx/sites-enabled/$DOMAIN_NAME
   \`\`\`

3. **Restart the services:**
   \`\`\`bash
   docker compose restart nginx
   \`\`\`

## Network Configuration

- Uses \`network_mode: host\` for both services
- Nginx proxies requests to the backend on port $BACKEND_PORT
- Rate limiting configured (10 requests/second)
- CORS headers included for API access

## Development

- Your existing Rust application files remain unchanged
- Nginx configurations can be customized in \`nginx/sites-available/\`
- Update Docker configuration as needed for your specific requirements

## Production Considerations

1. **Security**: Update SSL settings and security headers as needed
2. **Monitoring**: Add logging and monitoring solutions
3. **Backup**: Implement database and data backup strategies
4. **Scaling**: Consider load balancing for high traffic

## Troubleshooting

- **Check logs**: \`docker compose logs\`
- **Nginx config test**: \`docker compose exec nginx nginx -t\`
- **Reload Nginx**: \`docker compose exec nginx nginx -s reload\`
EOF

echo "✓ Created README-DOCKER-NGINX.md"

echo ""
echo "=========================================="
echo "Setup Complete!"
echo "=========================================="
echo ""
echo "Files created:"
echo "  ✓ Dockerfile"
echo "  ✓ compose.yaml"
echo "  ✓ nginx.conf"
echo "  ✓ nginx/sites-available/$DOMAIN_NAME"
echo "  ✓ nginx/sites-available/$DOMAIN_NAME.http-only"
if [ "$USE_SSL" = true ]; then
    echo "  ✓ nginx/sites-enabled/$DOMAIN_NAME (HTTPS active)"
else
    echo "  ✓ nginx/sites-enabled/$DOMAIN_NAME.http-only (HTTP active)"
fi
echo "  ✓ README-DOCKER-NGINX.md"
echo ""
echo "Next steps:"
echo "  1. Ensure your Rust application is ready"
echo "  2. Run: docker compose up --build"
if [ "$USE_SSL" = true ]; then
    echo "  3. Access: https://$DOMAIN_NAME"
    echo ""
    echo "Note: Make sure SSL certificates are properly configured"
else
    echo "  3. Access: http://$DOMAIN_NAME"
    echo ""
    echo "To enable HTTPS later, see README-DOCKER-NGINX.md"
fi
echo ""