---
type: knowledge
arxiv_id: "{{VALUE}}"
title: "{{VALUE}}"
authors: "{{VALUE}}"
year: "{{VALUE}}"
status: unverified
score: null
verdict: null
short_moniker: "{{VALUE}}"
tags: [knowledge]
processed: "{{date:YYYY-MM-DD}}"
source_md: "{{VALUE}}"

graph_edges:
  derives_from: []
  supersedes: []
  contradicts: []
---

# {{title}}

**Verdict**: {{verdict}} ({{score}}/10)

## Core Contribution

{{What is the actual delta this paper introduces? One paragraph.}}

## Mechanism

{{Input → Process → Output. How does it work, stripped of marketing.}}

## Experimental Setup

- Backbone: {{model}}
- Baselines: {{list}}
- Datasets: {{list}}
- Key result: {{metric + number}}

## Critical Flaws

{{What breaks? What's missing? What did they not test?}}

## Notes

{{Scratch space. Delete or promote to Thought Node when stabilized.}}
