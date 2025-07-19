#!/bin/bash

# Setup SSL for fuelcost.blackshadow.software
echo "🔐 Setting up SSL for fuelcost.blackshadow.software"

# Install certbot if not already installed
if ! command -v certbot &> /dev/null; then
    echo "📦 Installing certbot..."
    apt update
    apt install -y certbot python3-certbot-nginx
fi

# Stop nginx if running
echo "⏹️ Stopping nginx..."
docker-compose down nginx 2>/dev/null || true
systemctl stop nginx 2>/dev/null || true

# Get SSL certificate
echo "🔏 Obtaining SSL certificate..."
certbot certonly --standalone \
    --agree-tos \
    --non-interactive \
    --email admin@blackshadow.software \
    -d fuelcost.blackshadow.software

if [ $? -eq 0 ]; then
    echo "✅ SSL certificate obtained successfully!"
    
    # Switch to HTTPS nginx config
    echo "🔄 Switching to HTTPS nginx configuration..."
    cd /root/projects/fuel_cost_server
    rm -f nginx/sites-enabled/fuelcost.blackshadow.software
    ln -sf ../sites-available/fuelcost.blackshadow.software nginx/sites-enabled/fuelcost.blackshadow.software
    
    echo "🚀 SSL setup complete! You can now start with HTTPS enabled."
    echo "Run: docker-compose up -d"
else
    echo "❌ Failed to obtain SSL certificate"
    echo "Make sure your domain points to this server and port 80 is accessible"
    exit 1
fi