# Nginx Configuration for Rust REST API on VPS

This guide documents the complete process of deploying a Rust REST API (`fuel_cost_server`) on a VPS with Nginx as a reverse proxy.

## 📋 Prerequisites

- VPS running Ubuntu (Namecheap VPS in this case)
- Domain name: `YOUR_DOMAIN_URL`
- Subdomain: `YOUR_SERVER_DOMAIN_URL`
- VPS IP: `YOUR_SERVER_VPS_IP`
- Rust project: `fuel_cost_server`

## 🌐 Step 1: DNS Configuration

Set up the subdomain to point to your VPS:

1. Go to your domain registrar's DNS management panel
2. Add an A record:
   - **Name**: `fuelcost`
   - **Type**: A
   - **Value**: `YOUR_SERVER_VPS_IP`
   - **TTL**: 300 (or default)

This creates `YOUR_SERVER_DOMAIN_URL` pointing to your VPS.

## 🦀 Step 2: Rust Application Setup

### Build Your Rust Application

1. Install build tools on Ubuntu VPS:
```bash
sudo apt update
sudo apt install build-essential -y
```

2. Build your Rust project:
```bash
cd /root/servers  # or your project directory
cargo build --release
```

3. Verify the binary exists:
```bash
ls target/release/fuel_cost_server
```

### Configure Your Rust App

Ensure your Rust API binds to localhost with a specific port (8880 in this case):

```rust
// Example for Actix-web
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/api/health", web::get().to(health_check))
            // Add your other routes here
    })
    .bind("127.0.0.1:8880")? // Bind to localhost only
    .run()
    .await
}
```

## ⚙️ Step 3: Systemd Service Configuration

Create a systemd service to manage your Rust application:

### Create Service File

```bash
sudo nano /etc/systemd/system/fuel_cost_server.service
```

### Service Configuration

```ini
[Unit]
Description=FuelCost Rust API
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/servers
ExecStart=/root/servers/fuel_cost_server
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

### Enable and Start the Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable fuel_cost_server

# Start the service
sudo systemctl start fuel_cost_server

# Check status
sudo systemctl status fuel_cost_server
```

### Monitor Service Logs

```bash
# View real-time logs
journalctl -u fuel_cost_server -f

# Check if service is listening on correct port
sudo netstat -tlnp | grep :8880
```

## 🌍 Step 4: Nginx Installation and Configuration

### Install Nginx

```bash
# Install Nginx
sudo apt install nginx -y

# Start and enable Nginx
sudo systemctl start nginx
sudo systemctl enable nginx
```

### Create Nginx Site Configuration

```bash
sudo nano /etc/nginx/sites-available/YOUR_SERVER_DOMAIN_URL
```

### Nginx Configuration (HTTP Only)

```nginx
server {
    listen 80;
    server_name YOUR_SERVER_DOMAIN_URL;

    # Proxy to your Rust application
    location / {
        proxy_pass http://127.0.0.1:8880;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        proxy_read_timeout 86400;
    }
}
```

### Enable the Site

```bash
# Create symbolic link to enable the site
sudo ln -s /etc/nginx/sites-available/YOUR_SERVER_DOMAIN_URL /etc/nginx/sites-enabled/

# Test Nginx configuration
sudo nginx -t

# If test passes, reload Nginx
sudo systemctl reload nginx
```

## 🔥 Step 5: Firewall Configuration

```bash
# Allow HTTP traffic
sudo ufw allow 'Nginx Full'
sudo ufw allow ssh
sudo ufw enable
```

## 🧪 Step 6: Testing the Setup

### Test Rust Application Directly

```bash
# Test if your app is running on the correct port
curl http://127.0.0.1:8880/api/your-endpoint

# Check if port is listening
sudo ss -tlnp | grep :8880
```

### Test Through Nginx

```bash
# Test the domain
curl http://YOUR_SERVER_DOMAIN_URL/api/your-endpoint

# Or test a specific API endpoint
curl http://YOUR_SERVER_DOMAIN_URL/api/fuel-entries/f4038731-fa53-4df7-a56f-769de4b891f9
```

### Test from Browser

Open your browser and navigate to:
```
http://YOUR_SERVER_DOMAIN_URL/api/your-endpoint
```

## 📊 Step 7: Monitoring and Logs

### Check Service Status

```bash
# Nginx status
sudo systemctl status nginx

# Rust API status
sudo systemctl status fuel_cost_server
```

### View Logs

```bash
# Nginx error logs
sudo tail -f /var/log/nginx/error.log

# Nginx access logs
sudo tail -f /var/log/nginx/access.log

# Your Rust app logs
sudo journalctl -u fuel_cost_server -f
```

## 🔧 Troubleshooting

### Common Issues and Solutions

1. **404 Not Found Error**
   - Check if Rust app is running: `sudo systemctl status fuel_cost_server`
   - Verify port in Nginx config matches your app's port
   - Test direct connection: `curl http://127.0.0.1:8880`

2. **Port Mismatch**
   - Ensure Nginx `proxy_pass` points to correct port (8880 in this case)
   - Check what port your Rust app is actually using: `sudo netstat -tlnp | grep fuel_cost_server`

3. **Service Won't Start**
   - Check logs: `journalctl -u fuel_cost_server -f`
   - Verify binary permissions: `ls -la /root/servers/fuel_cost_server`
   - Ensure binary path in service file is correct

4. **DNS Issues**
   - Wait for DNS propagation (can take up to 24 hours)
   - Test with IP directly: `curl http://YOUR_SERVER_VPS_IP`

## 🔐 Optional: Adding SSL/HTTPS (Future)

When ready to add SSL, install Let's Encrypt:

```bash
# Install Certbot
sudo apt install certbot python3-certbot-nginx -y

# Generate SSL certificate
sudo certbot --nginx -d YOUR_SERVER_DOMAIN_URL

# Set up auto-renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

## 📁 Project Structure

```
/root/servers/
├── fuel_cost_server          # Compiled binary
├── Cargo.toml               # Rust project config
├── src/                     # Source code
└── target/                  # Build artifacts
    └── release/
        └── fuel_cost_server # Compiled binary
```

## 🚀 Deployment Commands Summary

Quick deployment checklist:

```bash
# 1. Build the project
cargo build --release

# 2. Restart the service
sudo systemctl restart fuel_cost_server

# 3. Check status
sudo systemctl status fuel_cost_server

# 4. Test the API
curl http://YOUR_SERVER_DOMAIN_URL/api/your-endpoint
```

## 📝 Notes

- The Rust application runs on port 8880 (localhost only)
- Nginx acts as a reverse proxy on port 80 (public)
- Service automatically restarts on failure
- Service starts automatically on system boot
- Currently using HTTP only (SSL can be added later)

## 🎯 Architecture Overview

```
Internet → YOUR_SERVER_DOMAIN_URL:80 → Nginx → 127.0.0.1:8880 → Rust API
```

---

**Success!** Your Rust API is now publicly accessible at `http://YOUR_SERVER_DOMAIN_URL`