# Deployment & Git Guide

## ☁️ VPS Deployment (Helius Friendly)
For optimal performance, use a VPS in **New York (NY)** or **Amsterdam**.

1. **Connect** to your VPS.
2. **Install Node.js & Git**:
   ```bash
   sudo apt update && sudo apt install nodejs npm git -y
   ```
3. **Upload Code**: Use Git or SFTP.
4. **Environment Setup**:
   ```bash
   npm install
   cp .env.example .env
   # Add your PRIVATE_KEY to .env
   npm run build
   ```
5. **Keep Alive**: Use PM2 to run the bot 24/7.
   ```bash
   npm install -g pm2
   pm2 start dist/index.js --name "sniper"
   ```

## 🐙 Git Instructions

### Push to your Repository
```bash
git init
git add .
git commit -m "Solana Sniper Bot implementation with Helius"
git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPO.git
git push origin master
```

### Push to Client Repo (Collaborator)
```bash
git remote add client https://github.com/Joebryace4/Solana-trading-bot.git
git push client master
```

## 📁 Drive / Transfer Fix
**Never** upload `node_modules`. It is huge and causes errors.
- **Before uploading**: Delete `node_modules`.
- **Zip the folder**: Right-click `solana-sniper-client` -> Compress.
- **After downloading**: Run `npm install` again.
