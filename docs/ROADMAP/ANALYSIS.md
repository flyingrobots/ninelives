# DAG Analysis Report

## 1. Metrics
- **Total Tasks**: 143
- **Total Edges**: 164
- **Max Depth (Critical Path Length)**: 18
- **Max Width (Max Concurrency)**: 16

## 2. Critical Path
> The longest sequence of dependent tasks. Any delay here delays the project.

P9.01a -> P9.01b -> P9.01c -> P9.01d -> P9.01e -> P9.02a -> P9.02b -> P9.02c -> P9.02d -> P9.02e -> P9.03a -> P9.03b -> P9.03c -> P9.03d -> P9.03e -> P9.04a -> P9.04b -> P9.04c -> P9.04d


## 3. Execution Schedule (Antichains)
> Tasks grouped by depth. Tasks in the same Level *can* be executed in parallel (assuming resources).

### Level 0 (11 tasks)
- P10.01a 
- P2.01 
- P2.07 
- P2.121 
- P3.01a 
- P4.01a 
- P5.01a 
- P6.01a 
- P7.01a 
- P8.01a 
- P9.01a 🔥

### Level 1 (16 tasks)
- P1.09 
- P10.01b 
- P2.02 
- P2.03 
- P2.04 
- P2.05 
- P2.06 
- P2.08 
- P3.01b 
- P4.01b 
- P5.01b 
- P6.01b 
- P7.01b 
- P8.01b 
- P8.05a 
- P9.01b 🔥

### Level 2 (10 tasks)
- P10.01c 
- P2.09 
- P3.02a 
- P4.02a 
- P5.01c 
- P6.01c 
- P7.01c 
- P8.01c 
- P8.05b 
- P9.01c 🔥

### Level 3 (16 tasks)
- P10.01d 
- P2.10 
- P2.11 
- P2.12 
- P2.13 
- P2.14a 
- P2.16a 
- P2.17 
- P4.03a 
- P5.02a 
- P6.01d 
- P7.01g 
- P7.04a 
- P8.02a 
- P8.05c 
- P9.01d 🔥

### Level 4 (10 tasks)
- P10.02a 
- P2.14b 
- P2.16b 
- P4.03b 
- P5.02b 
- P6.01e 
- P7.01d 
- P7.04b 
- P8.02b 
- P9.01e 🔥

### Level 5 (10 tasks)
- P10.02b 
- P2.15 
- P2.18 
- P4.04a 
- P5.02c 
- P6.02a 
- P7.01e 
- P7.04c 
- P8.02c 
- P9.02a 🔥

### Level 6 (7 tasks)
- P10.02c 
- P4.04b 
- P5.02d 
- P6.02b 
- P7.01f 
- P8.02d 
- P9.02b 🔥

### Level 7 (6 tasks)
- P10.02d 
- P5.03a 
- P6.02c 
- P7.02a 
- P8.02e 
- P9.02c 🔥

### Level 8 (6 tasks)
- P10.03a 
- P5.03b 
- P6.02d 
- P7.02b 
- P8.03a 
- P9.02d 🔥

### Level 9 (7 tasks)
- P10.03b 
- P3.02b 
- P5.03c 
- P6.03a 
- P7.02c 
- P8.03b 
- P9.02e 🔥

### Level 10 (7 tasks)
- P10.03c 
- P3.03a 
- P3.06 
- P6.03b 
- P7.02d 
- P8.03c 
- P9.03a 🔥

### Level 11 (8 tasks)
- P10.03d 
- P2.19 
- P3.03b 
- P3.07a 
- P6.03c 
- P7.03a 
- P8.03d 
- P9.03b 🔥

### Level 12 (10 tasks)
- P10.03e 
- P2.20 
- P3.04a 
- P3.05a 
- P3.07b 
- P5.04a 
- P6.03d 
- P7.03b 
- P8.03e 
- P9.03c 🔥

### Level 13 (8 tasks)
- P10.04a 
- P3.04b 
- P3.05b 
- P3.07c 
- P5.04b 
- P6.04a 
- P8.04a 
- P9.03d 🔥

### Level 14 (5 tasks)
- P10.04b 
- P5.04c 
- P6.04b 
- P8.04b 
- P9.03e 🔥

### Level 15 (2 tasks)
- P10.04c 
- P9.04a 🔥

### Level 16 (2 tasks)
- P10.04d 
- P9.04b 🔥

### Level 17 (1 tasks)
- P9.04c 🔥

### Level 18 (1 tasks)
- P9.04d 🔥

## 4. Topological Sort Order
> One valid linear execution sequence.

P7.01a, P6.01a, P8.01a, P5.01a, P10.01a, P2.07, P3.01a, P4.01a, P9.01a, P2.121, P2.01, P7.01b, P6.01b, P8.01b, P8.05a, P5.01b, P10.01b, P1.09, P2.08, P3.01b, P9.01b, P4.01b, P2.02, P2.03, P2.04, P2.05, P2.06, P7.01c, P6.01c, P8.01c, P8.05b, P5.01c, P10.01c, P2.09, P3.02a, P9.01c, P4.02a, P7.01g, P7.04a, P6.01d, P8.02a, P8.05c, P5.02a, P10.01d, P2.10, P2.11, P2.12, P2.13, P2.14a, P2.16a, P2.17, P9.01d, P4.03a, P7.01d, P7.04b, P6.01e, P8.02b, P5.02b, P10.02a, P2.14b, P2.16b, P9.01e, P4.03b, P7.01e, P7.04c, P6.02a, P8.02c, P5.02c, P10.02b, P2.15, P2.18, P9.02a, P4.04a, P7.01f, P6.02b, P8.02d, P5.02d, P10.02c, P9.02b, P4.04b, P7.02a, P6.02c, P8.02e, P5.03a, P10.02d, P9.02c, P7.02b, P6.02d, P8.03a, P5.03b, P10.03a, P9.02d, P7.02c, P3.02b, P6.03a, P8.03b, P5.03c, P10.03b, P9.02e, P7.02d, P3.03a, P3.06, P6.03b, P8.03c, P10.03c, P9.03a, P7.03a, P3.03b, P3.07a, P6.03c, P2.19, P8.03d, P10.03d, P9.03b, P7.03b, P3.04a, P3.05a, P3.07b, P6.03d, P5.04a, P2.20, P8.03e, P10.03e, P9.03c, P3.04b, P3.05b, P3.07c, P6.04a, P5.04b, P8.04a, P10.04a, P9.03d, P6.04b, P5.04c, P8.04b, P10.04b, P9.03e, P10.04c, P9.04a, P10.04d, P9.04b, P9.04c, P9.04d
