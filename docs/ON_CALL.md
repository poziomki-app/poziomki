# On-Call Procedures

How to handle incidents and who responds when things go wrong.

**Related:**
- [INCIDENT-RESPONSE.md](./security/INCIDENT-RESPONSE.md) — Security-specific incidents
- [DEPLOYMENT.md](./DEPLOYMENT.md) — Infrastructure details

---

## Team Context

**Reality check:** Poziomki is run by 3 students. We don't have 24/7 coverage.

| Principle | Policy |
|-----------|--------|
| **Studies first** | During exams, response times increase |
| **No burnout** | No one is obligated to respond at 3 AM |
| **Transparency** | If we're slow, we communicate that |
| **Automation** | Automate recovery where possible |

---

## Contact Channels

### Internal (Team)

| Channel | Use For | Response Time |
|---------|---------|---------------|
| Signal group | Urgent issues | Best effort |
| GitHub Issues | Non-urgent bugs | 24-48 hours |
| Email | External reports | 48 hours |

### External (Users)

| Channel | Address | Monitored |
|---------|---------|-----------|
| Security | security@poziomki.app | Daily |
| General | contact@poziomki.app | Daily |
| Safety | safety@poziomki.app | Daily |

---

## Severity Levels

| Level | Definition | Response Target | Example |
|-------|------------|-----------------|---------|
| **P0 Critical** | Service down, data breach | ASAP (within hours) | Database down, auth broken |
| **P1 High** | Major feature broken | Same day | Chat not working, can't create events |
| **P2 Medium** | Feature degraded | 24-48 hours | Slow performance, minor bugs |
| **P3 Low** | Cosmetic, minor issues | Next week | Typos, UI glitches |

---

## On-Call Rotation

### Schedule
With 3 people, formal rotation isn't practical. Instead:

| Period | Primary | Backup |
|--------|---------|--------|
| Week 1 | Person A | Person B |
| Week 2 | Person B | Person C |
| Week 3 | Person C | Person A |

**Exam periods:** Whoever is available. Announce in Signal if unavailable.

### Responsibilities
**Primary:**
- First to respond to alerts
- Triage incoming issues
- Escalate if needed

**Backup:**
- Available if primary unreachable
- Help with major incidents

---

## Monitoring & Alerts

### What We Monitor

| Service | Check | Alert Threshold |
|---------|-------|-----------------|
| API health | `/health` endpoint | 3 consecutive failures |
| Database | Connection test | Any failure |
| CDN | File accessibility | 5 failures/minute |
| SSL certificates | Expiry check | 14 days before expiry |

### Alert Channels

| Severity | Channel |
|----------|---------|
| P0 | Signal group + email |
| P1 | Signal group |
| P2+ | GitHub issue |

### Monitoring Tools

Current: Basic health checks via uptime monitoring service
Planned: Prometheus + Grafana (see OBSERVABILITY.md)

---

## Incident Response

### Step 1: Acknowledge
- Respond in Signal: "Looking at it"
- Prevents duplicate work

### Step 2: Assess
- What's the impact? (users affected, data at risk)
- What's the severity?
- Can users work around it?

### Step 3: Communicate
**If P0/P1:**
- Post status update (if we have status page)
- Consider in-app banner for major outages

### Step 4: Mitigate
- Focus on restoring service first
- Root cause analysis later
- Document actions taken

### Step 5: Resolve
- Confirm service restored
- Update status
- Schedule post-mortem for P0/P1

---

## Common Scenarios

### API Down

**Symptoms:** Health check failing, mobile app showing errors

**Quick checks:**
```bash
# SSH to server
ssh admin@poziomki.app

# Check if API is running
systemctl status poziomki-api

# Check logs
journalctl -u poziomki-api -n 100

# Restart if needed
sudo systemctl restart poziomki-api
```

**Common causes:**
- Out of memory → Restart, check for memory leaks
- Database connection lost → Check PostgreSQL, restart
- Deployment failed → Rollback to previous version

### Database Issues

**Symptoms:** Slow queries, connection errors

**Quick checks:**
```bash
# Check PostgreSQL status
systemctl status postgresql

# Check connections
sudo -u postgres psql -c "SELECT count(*) FROM pg_stat_activity;"

# Check disk space
df -h
```

**Common causes:**
- Too many connections → Check connection pooling
- Disk full → Clean up, add storage
- Slow queries → Check pg_stat_statements

### CDN/Storage Issues

**Symptoms:** Images not loading, upload failures

**Quick checks:**
```bash
# Check MinIO status
systemctl status minio

# Test access
curl -I https://cdn.poziomki.app/health
```

**Common causes:**
- MinIO down → Restart service
- Disk full → Clean up old files
- Certificate expired → Renew via Caddy

### High Traffic

**Symptoms:** Slow responses, rate limiting triggered

**Quick actions:**
1. Check if it's legitimate traffic or attack
2. If attack: block IPs at Caddy level
3. If legitimate: consider scaling (see CACHING.md)

---

## Runbooks

### Restart API
```bash
sudo systemctl restart poziomki-api
# Wait 30 seconds
curl http://localhost:3000/health
```

### Rollback Deployment
```bash
cd /opt/poziomki
git log --oneline -5  # Find previous good commit
git checkout <commit>
bun install
sudo systemctl restart poziomki-api
```

### Database Backup (Emergency)
```bash
pg_dump -U poziomki poziomki > /backups/emergency-$(date +%Y%m%d-%H%M%S).sql
```

### Clear Rate Limits
```bash
# If using Redis/Dragonfly
redis-cli KEYS "ratelimit:*" | xargs redis-cli DEL
```

### Block Abusive IP
```bash
# Add to Caddy blocklist
echo "1.2.3.4" >> /etc/caddy/blocked-ips.txt
sudo systemctl reload caddy
```

---

## Post-Incident

### For P0/P1 Incidents

Within 48 hours:
1. **Timeline:** What happened, when
2. **Impact:** Users affected, duration
3. **Root cause:** Why it happened
4. **Action items:** How to prevent recurrence

### Template
```markdown
## Incident: [Title]
**Date:** YYYY-MM-DD
**Duration:** X hours
**Severity:** P0/P1
**Author:** [Name]

### Summary
One paragraph describing what happened.

### Timeline
- HH:MM - Alert triggered
- HH:MM - Investigation started
- HH:MM - Root cause identified
- HH:MM - Fix deployed
- HH:MM - Service restored

### Impact
- X users affected
- Y minutes of downtime
- Z failed requests

### Root Cause
What actually caused the issue.

### Resolution
What we did to fix it.

### Action Items
- [ ] Preventive measure 1
- [ ] Preventive measure 2
```

---

## Escalation

### When to Escalate

| Situation | Escalate To |
|-----------|-------------|
| Can't diagnose issue | Another team member |
| Security breach suspected | All team members + security@poziomki.app |
| Legal/compliance issue | All team members |
| Need infrastructure access | Person with server access |

### External Escalation

| Provider | Contact | For |
|----------|---------|-----|
| Hetzner | Support portal | Server issues |
| Domain registrar | Support portal | DNS issues |
| Email provider | Support portal | Email delivery |

---

## Availability Expectations

### Realistic SLA (Internal)

| Metric | Target | Notes |
|--------|--------|-------|
| Uptime | 99% | ~7 hours downtime/month acceptable |
| P0 response | < 4 hours | During waking hours |
| P1 response | < 24 hours | Next business day |

### Reduced Availability Periods
- Exam sessions (January-February, June)
- Holidays
- Conference attendance

**During these periods:**
- Increase alert thresholds
- Accept longer response times
- Communicate delays to users if needed

---

## Checklist: Before Going On-Call

- [ ] Server access works (SSH key, sudo)
- [ ] Monitoring dashboard accessible
- [ ] Signal notifications enabled
- [ ] Know location of runbooks
- [ ] Know who's backup
- [ ] Phone charged

---

*Last updated: 2026-02-03*
