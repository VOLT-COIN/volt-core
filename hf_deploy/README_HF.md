# 🐳 Hugging Face Deployment Guide

This allows you to verify a FREE, STRONG server (16GB RAM) with Open Ports.

## Steps:

1. **Go to Hugging Face:**
   - Create a **New Space**.
   - **Name:** `volt-node` (or anything).
   - **SDK:** Select **Docker**.
   - **Hardware:** FREE (Default).

2. **Upload Files:**
   - Upload the `Dockerfile` and `run.sh` from this folder (`hf_deploy`) to the Space.
   - (You can drag and drop them in the "Files" tab of your Space).

3. **Wait for Build:**
   - The Space will say "Building...". This takes ~5-10 minutes (compiling Rust).

4. **Claim Your Tunnel:**
   - Click "Logs" in the Space.
   - You will see a **Playit Claim URL** (e.g., https://playit.gg/claim/xyz...).
   - Click it to link this server to your Playit account.
   - Create a generic TCP tunnel mapping to `127.0.0.1:6000`.

5. **Success!**
   - You now have a public address (e.g., `fast-wolf.playit.gg:12345`) that points to your Node.
   - Use this address in your wallet or other nodes to connect!
