# Incident Response Plan

**Document Owner:** Security Team
**Last Updated:** 2026-02-02
**Review Frequency:** Quarterly

---

## 1. Purpose & Scope

This document establishes procedures for detecting, responding to, and recovering from security incidents affecting Poziomki systems and user data.

### Scope

- Poziomki API and backend services
- Mobile application (iOS, Android)
- Database systems (PostgreSQL)
- Object storage (MinIO)
- CDN and reverse proxy (Caddy)
- User personal data (GDPR scope)

### Out of Scope

- Physical security incidents
- Third-party service provider incidents (coordinate with vendor)
- Employee HR matters (coordinate with management)

---

## 2. Incident Classification

### Severity Levels

| Level | Name | Description | Examples |
|-------|------|-------------|----------|
| **P1** | Critical | Active breach, data exfiltration, service unavailable | Database breach, ransomware, complete outage |
| **P2** | High | Significant risk, limited impact, service degraded | Unauthorized access attempt, partial outage |
| **P3** | Medium | Moderate risk, no immediate impact | Suspicious activity, vulnerability discovered |
| **P4** | Low | Minor risk, informational | Failed login attempts, policy violation |

### Incident Categories

| Category | Description | GDPR Notifiable? |
|----------|-------------|------------------|
| **Data Breach** | Unauthorized access to personal data | Yes (if risk to rights) |
| **System Compromise** | Unauthorized system access | Potentially |
| **Service Disruption** | DoS, outage, degraded performance | No |
| **Account Compromise** | Individual user account takeover | Potentially |
| **Malware** | Malicious software detected | Potentially |
| **Insider Threat** | Unauthorized internal access | Yes (if data affected) |
| **Policy Violation** | Security policy non-compliance | No |

---

## 3. Response Team

### Roles & Responsibilities

| Role | Responsibilities | Contact |
|------|------------------|---------|
| **Incident Commander** | Overall coordination, decisions, communication | [Primary contact] |
| **Technical Lead** | Investigation, containment, remediation | [Technical contact] |
| **Communications Lead** | Internal/external communications, user notifications | [Comms contact] |
| **Legal/Compliance** | GDPR assessment, regulatory notifications | [Legal contact] |

### RACI Matrix

| Activity | Commander | Tech Lead | Comms | Legal |
|----------|-----------|-----------|-------|-------|
| Incident detection | I | R | I | I |
| Initial triage | A | R | I | I |
| Containment | A | R | I | C |
| Investigation | A | R | I | C |
| User notification | A | C | R | A |
| Regulatory notification | A | C | C | R |
| Recovery | A | R | I | I |
| Post-incident review | A | R | C | C |

*R=Responsible, A=Accountable, C=Consulted, I=Informed*

### Escalation Path

```
P4 → Technical Lead → Incident Commander (if escalates)
P3 → Technical Lead → Incident Commander
P2 → Incident Commander → Full Team
P1 → Incident Commander → Full Team → Legal/Compliance
```

---

## 4. Response Procedures

### Phase 1: Detection & Triage (0-30 minutes)

**Objective:** Confirm incident, assess severity, activate response.

#### Steps

1. **Receive Alert**
   - Source: Monitoring alert, user report, security tool, team member
   - Log: Date, time, source, initial description

2. **Initial Assessment**
   - What systems are affected?
   - Is there evidence of data access?
   - Is the service operational?
   - Is the threat ongoing?

3. **Classify Severity**
   - Use classification matrix above
   - When in doubt, classify higher

4. **Activate Response**
   - P1/P2: Immediate team notification
   - P3/P4: Log and assign within business hours

5. **Document**
   - Create incident ticket
   - Record timeline from this point forward

#### Checklist

```markdown
- [ ] Incident confirmed (not false positive)
- [ ] Severity level assigned: P__
- [ ] Category assigned: __________
- [ ] Incident Commander notified (P1/P2)
- [ ] Incident ticket created: #____
- [ ] Initial timeline documented
```

### Phase 2: Containment (30 min - 4 hours)

**Objective:** Prevent further damage, preserve evidence.

#### Short-Term Containment

| Scenario | Action |
|----------|--------|
| Compromised server | Isolate from network, do NOT shut down |
| Database breach | Revoke compromised credentials, enable read-only |
| Account compromise | Force logout, revoke sessions, require password reset |
| Active attack | Enable enhanced rate limiting, block IP ranges |
| Malware | Isolate system, disable network access |

#### Evidence Preservation

**CRITICAL:** Preserve evidence before making changes.

```bash
# Capture system state
docker logs api > /evidence/api-logs-$(date +%s).txt
docker logs postgres > /evidence/postgres-logs-$(date +%s).txt

# Database audit (if enabled)
pg_dump -t audit_log poziomki > /evidence/audit-$(date +%s).sql

# Network connections
ss -tunapl > /evidence/connections-$(date +%s).txt

# Running processes
ps auxf > /evidence/processes-$(date +%s).txt
```

#### Checklist

```markdown
- [ ] Evidence preserved before changes
- [ ] Short-term containment implemented
- [ ] Affected systems isolated (if needed)
- [ ] Credentials rotated (if compromised)
- [ ] Attack vector blocked (if identified)
- [ ] Service status assessed
```

### Phase 3: Investigation (2-48 hours)

**Objective:** Determine root cause, scope, and impact.

#### Investigation Questions

1. **Timeline**
   - When did the incident start?
   - How was it detected?
   - What is the current status?

2. **Attack Vector**
   - How did the attacker gain access?
   - What vulnerability was exploited?
   - Was it internal or external?

3. **Scope**
   - Which systems were affected?
   - Which data was accessed/exfiltrated?
   - How many users impacted?

4. **Attribution**
   - Source IP addresses
   - Attack patterns
   - Known threat actor indicators

#### Log Sources

| Source | Location | Useful For |
|--------|----------|------------|
| API logs | `docker logs api` | Request patterns, errors |
| Auth logs | `session` table, API logs | Login attempts, token usage |
| Database logs | `docker logs postgres` | Query patterns, errors |
| Caddy logs | `/var/log/caddy/` | Request origins, errors |
| Audit table | `audit_log` table | Sensitive operations |

#### Checklist

```markdown
- [ ] Attack timeline established
- [ ] Attack vector identified
- [ ] Affected systems documented
- [ ] Affected data types identified
- [ ] Number of affected users estimated
- [ ] Root cause determined
```

### Phase 4: Eradication & Recovery (4-72 hours)

**Objective:** Remove threat, restore service, prevent recurrence.

#### Eradication Steps

1. **Remove malware/backdoors** (if present)
2. **Patch vulnerabilities** exploited
3. **Rotate all credentials** that may be compromised
4. **Review and harden configurations**
5. **Update security controls**

#### Recovery Steps

1. **Verify system integrity** before restoration
2. **Restore from clean backups** if needed
3. **Implement additional monitoring**
4. **Gradual service restoration**
5. **Verify normal operation**

#### Checklist

```markdown
- [ ] Threat removed from all systems
- [ ] Vulnerability patched
- [ ] Credentials rotated
- [ ] Systems restored from clean state
- [ ] Enhanced monitoring in place
- [ ] Service fully operational
- [ ] Normal operations verified
```

### Phase 5: Notification (Within GDPR timeline)

**Objective:** Notify authorities and users as required.

#### GDPR Requirements

| Notification | Timeline | When Required |
|--------------|----------|---------------|
| **Supervisory Authority (CNIL)** | 72 hours | Risk to rights and freedoms |
| **Data Subjects (Users)** | Without undue delay | High risk to rights and freedoms |

#### CNIL Notification Process

1. Go to: https://www.cnil.fr/fr/notifier-une-violation-de-donnees-personnelles
2. Required information:
   - Nature of the breach
   - Categories and approximate number of data subjects
   - Categories and approximate number of records
   - Contact details for DPO/point of contact
   - Likely consequences
   - Measures taken or proposed

#### User Notification Template

```
Subject: Security Notification - Action Required

Dear [User],

We are writing to inform you of a security incident that affected your Poziomki account.

**What happened:**
[Clear, non-technical description of the incident]

**When it happened:**
[Date range]

**What information was involved:**
[List specific data types: email, profile information, etc.]

**What we are doing:**
[Steps taken to address the incident]

**What you can do:**
- Change your password immediately
- Review your account activity
- Be alert for suspicious emails or messages

**Contact us:**
If you have questions, please contact security@poziomki.app

We apologize for any concern this may cause.

The Poziomki Team
```

#### Checklist

```markdown
- [ ] GDPR notification required? Yes/No
- [ ] CNIL notified within 72 hours (if required)
- [ ] User notification sent (if required)
- [ ] Internal stakeholders notified
- [ ] Documentation complete for regulatory review
```

### Phase 6: Post-Incident Review (Within 2 weeks)

**Objective:** Learn from incident, improve defenses.

#### Post-Incident Review Meeting

**Attendees:** All incident response team members

**Agenda:**
1. Incident timeline review
2. What worked well
3. What could be improved
4. Root cause analysis
5. Action items for improvement

#### Questions to Address

- Was detection timely?
- Was the response effective?
- Were procedures followed?
- Were communications adequate?
- What could prevent recurrence?

#### Deliverables

1. **Incident Report** (template below)
2. **Lessons Learned Document**
3. **Action Items with Owners and Deadlines**
4. **Updated Procedures** (if needed)

---

## 5. Templates

### Incident Report Template

```markdown
# Incident Report: [Incident ID]

## Executive Summary
[2-3 sentence summary for leadership]

## Incident Details
- **Incident ID:** INC-YYYY-MM-###
- **Severity:** P1/P2/P3/P4
- **Category:** [Data Breach/System Compromise/etc.]
- **Status:** [Open/Contained/Resolved/Closed]

## Timeline
| Date/Time | Event |
|-----------|-------|
| YYYY-MM-DD HH:MM | [Event description] |

## Impact Assessment
- **Systems Affected:** [List]
- **Data Types Affected:** [List]
- **Users Affected:** [Number/Description]
- **Service Disruption:** [Duration/Type]

## Root Cause
[Description of root cause]

## Response Actions
1. [Action taken]
2. [Action taken]

## Lessons Learned
- [Lesson 1]
- [Lesson 2]

## Recommendations
- [ ] [Recommendation 1] - Owner: [Name] - Due: [Date]
- [ ] [Recommendation 2] - Owner: [Name] - Due: [Date]

## Appendices
- Evidence files
- Communication logs
- Related tickets
```

### Communication Templates

#### Internal Escalation (P1/P2)

```
SECURITY INCIDENT - [SEVERITY]

Time: [Date/Time]
Status: [Active/Contained]

Summary: [1-2 sentences]

Affected: [Systems/Users]

Action Required: [Specific action]

Contact: [Incident Commander contact]

Updates: [Channel/frequency]
```

#### Status Update

```
INCIDENT UPDATE - [Incident ID]

Time: [Date/Time]
Status: [Current status]

Since last update:
- [Update 1]
- [Update 2]

Next steps:
- [Step 1]
- [Step 2]

Next update: [Time]
```

---

## 6. Contact Information

| Role | Name | Phone | Email |
|------|------|-------|-------|
| Incident Commander | [Name] | [Phone] | [Email] |
| Technical Lead | [Name] | [Phone] | [Email] |
| Communications Lead | [Name] | [Phone] | [Email] |
| Legal/Compliance | [Name] | [Phone] | [Email] |

### External Contacts

| Organization | Purpose | Contact |
|--------------|---------|---------|
| CNIL | Regulatory notification | https://www.cnil.fr |
| OVHcloud Support | Infrastructure issues | Support portal |
| Legal Counsel | Legal advice | [Contact] |

---

## 7. Testing & Maintenance

### Quarterly Tasks

- [ ] Review and update contact information
- [ ] Verify backup restoration procedures
- [ ] Test alerting systems
- [ ] Review recent incidents for patterns

### Annual Tasks

- [ ] Full tabletop exercise
- [ ] Review and update procedures
- [ ] Training for all team members
- [ ] External assessment (optional)

---

## 8. Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-02-02 | Security Team | Initial version |

---

## Appendix A: Quick Reference Card

### Incident Response Checklist

```
□ 1. DETECT - Confirm incident, classify severity
□ 2. CONTAIN - Preserve evidence, limit damage
□ 3. INVESTIGATE - Determine scope and root cause
□ 4. ERADICATE - Remove threat, patch vulnerability
□ 5. RECOVER - Restore service, verify integrity
□ 6. NOTIFY - Authorities (72h) and users (if required)
□ 7. REVIEW - Document lessons, improve procedures
```

### Key Timelines

| Milestone | Target |
|-----------|--------|
| Initial response | 30 minutes |
| Containment | 4 hours |
| CNIL notification | 72 hours (if required) |
| Post-incident review | 2 weeks |
