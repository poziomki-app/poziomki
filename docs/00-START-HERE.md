# Poziomki Documentation

Welcome to the Poziomki documentation. This guide helps you find the right documentation for your needs.

---

## Quick Links

| I want to... | Go to |
|--------------|-------|
| Get started developing | [README.md](../README.md) |
| Understand the architecture | [ARCHITECTURE.md](./ARCHITECTURE.md) |
| Handle API errors | [ERROR_CODES.md](./ERROR_CODES.md) |
| Deploy to production | [DEPLOYMENT.md](./DEPLOYMENT.md) |
| Deploy to iOS | [IOS_DEPLOYMENT.md](./IOS_DEPLOYMENT.md) |
| Check performance baselines | [PERFORMANCE.md](./PERFORMANCE.md) |
| Handle an incident | [ON_CALL.md](./ON_CALL.md) |
| Report a security issue | [SECURITY.md](../SECURITY.md) |
| Contribute code | [CONTRIBUTING.md](../CONTRIBUTING.md) |

---

## Documentation Index

### Getting Started

| Document | Description |
|----------|-------------|
| [README.md](../README.md) | Project overview, quick start, tech stack |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | How to contribute, code standards |
| [CLAUDE.md](../CLAUDE.md) | Developer guide, API patterns, project rules |

### Architecture & Design

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | System design, component diagrams, data flow |
| [apps/api/README.md](../apps/api/README.md) | Backend API structure and patterns |
| [apps/mobile/README.md](../apps/mobile/README.md) | Mobile app structure and patterns |
| [packages/db/README.md](../packages/db/README.md) | Database schema and migrations |
| [packages/core/README.md](../packages/core/README.md) | Shared utilities |

### Legal & Policy

| Document | Description |
|----------|-------------|
| [PRIVACY_POLICY.md](./PRIVACY_POLICY.md) | Privacy policy (GDPR compliant) |
| [TERMS_OF_SERVICE.md](./TERMS_OF_SERVICE.md) | Terms of service |
| [COMMUNITY_GUIDELINES.md](./COMMUNITY_GUIDELINES.md) | Community rules and moderation |
| [DATA_RETENTION.md](./DATA_RETENTION.md) | Data retention schedules |

### Security

| Document | Description |
|----------|-------------|
| [SECURITY.md](../SECURITY.md) | Security policy, vulnerability reporting |
| [SECURITY_ISSUES.md](../SECURITY_ISSUES.md) | Security audit, known issues, remediation |
| [security/THREAT-MODEL.md](./security/THREAT-MODEL.md) | Threat analysis, risk assessment |
| [security/INCIDENT-RESPONSE.md](./security/INCIDENT-RESPONSE.md) | Incident response procedures |

### API Reference

| Document | Description |
|----------|-------------|
| `/api/docs` | Interactive API documentation (OpenAPI + Scalar) |
| [ERROR_CODES.md](./ERROR_CODES.md) | API error codes catalog for mobile devs |

### Operations

| Document | Description |
|----------|-------------|
| [DEPLOYMENT.md](./DEPLOYMENT.md) | Production deployment guide |
| [IOS_DEPLOYMENT.md](./IOS_DEPLOYMENT.md) | iOS App Store deployment |
| [ON_CALL.md](./ON_CALL.md) | On-call procedures and runbooks |
| [PERFORMANCE.md](./PERFORMANCE.md) | Performance baselines, benchmarking |
| [CACHING.md](../CACHING.md) | Caching strategy, scalability, GDPR |
| [OBSERVABILITY.md](../OBSERVABILITY.md) | Monitoring, metrics, logging |

### Team & Process

| Document | Description |
|----------|-------------|
| [AGENTS.md](../AGENTS.md) | Team roles and responsibilities |

### Research

| Document | Description |
|----------|-------------|
| [docs/research-2026/](./research-2026/) | Research documents (Polish) |

---

## By Role

### For New Contributors

1. Start with [README.md](../README.md) for setup
2. Read [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines
3. Review [CLAUDE.md](../CLAUDE.md) for coding standards
4. Check [ARCHITECTURE.md](./ARCHITECTURE.md) for system overview

### For Security Reviewers

1. [SECURITY_ISSUES.md](../SECURITY_ISSUES.md) - Current audit findings
2. [security/THREAT-MODEL.md](./security/THREAT-MODEL.md) - Threat analysis
3. [security/INCIDENT-RESPONSE.md](./security/INCIDENT-RESPONSE.md) - Response procedures
4. [SECURITY.md](../SECURITY.md) - Disclosure policy

### For DevOps / Operations

1. [DEPLOYMENT.md](./DEPLOYMENT.md) - Production setup
2. [PERFORMANCE.md](./PERFORMANCE.md) - Baselines and benchmarking
3. [CACHING.md](../CACHING.md) - Performance and scaling
4. [OBSERVABILITY.md](../OBSERVABILITY.md) - Monitoring setup

### For API Developers

1. [apps/api/README.md](../apps/api/README.md) - API structure
2. [ERROR_CODES.md](./ERROR_CODES.md) - Error codes reference
3. [CLAUDE.md](../CLAUDE.md) - API patterns
4. [packages/db/README.md](../packages/db/README.md) - Database access

### For Mobile Developers

1. [apps/mobile/README.md](../apps/mobile/README.md) - Mobile structure
2. [ERROR_CODES.md](./ERROR_CODES.md) - API error handling
3. [CLAUDE.md](../CLAUDE.md) - Coding standards

---

## Document Maintenance

| Document | Owner | Review Frequency |
|----------|-------|------------------|
| README.md | Engineering | On major changes |
| ARCHITECTURE.md | Engineering | Quarterly |
| ERROR_CODES.md | Engineering | On API changes |
| PERFORMANCE.md | Engineering | Monthly |
| PRIVACY_POLICY.md | Legal | Annually or on changes |
| TERMS_OF_SERVICE.md | Legal | Annually or on changes |
| COMMUNITY_GUIDELINES.md | Community | Quarterly |
| DATA_RETENTION.md | Legal/Engineering | Annually |
| SECURITY_ISSUES.md | Security | Monthly |
| THREAT-MODEL.md | Security | Quarterly |
| INCIDENT-RESPONSE.md | Security | Quarterly |
| ON_CALL.md | Operations | Quarterly |
| DEPLOYMENT.md | Operations | On changes |
| IOS_DEPLOYMENT.md | Operations | On changes |
| CACHING.md | Engineering | On scaling needs |

---

## Need Help?

- **Technical questions**: Open a GitHub issue
- **Security issues**: Email security@poziomki.app (see [SECURITY.md](../SECURITY.md))
- **Contribution questions**: See [CONTRIBUTING.md](../CONTRIBUTING.md)

---

*Last updated: 2026-02-03*
