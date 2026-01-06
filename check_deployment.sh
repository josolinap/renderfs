#!/usr/bin/env bash

# Render deployment monitoring script
RENDER_API_KEY="rnd_Q3A26duzUHlHAkf8mpjR7JUExetn"
SERVICE_ID="renderfs"  # This might need to be adjusted

echo "🚀 Checking Render deployment status..."
echo "=================================="

# Check if service exists and get status
curl -s -H "Authorization: Bearer $RENDER_API_KEY" \
     "https://api.render.com/v1/services" | \
     grep -E '"name":|"status":|"url":|"currentDeployId":' | \
     head -20

echo ""
echo "📊 Recent deployment logs..."
echo "==========================="

# Get deployment logs (this would require the deployment ID)
# For now, let's check the service status again
curl -s -H "Authorization: Bearer $RENDER_API_KEY" \
     "https://api.render.com/v1/services" | \
     jq '.[] | select(.name | contains("renderfs")) | {name, status, url, createdAt}' 2>/dev/null || \
     echo "⚠️  jq not available, showing raw response..."

echo ""
echo "✅ Deployment check completed!"