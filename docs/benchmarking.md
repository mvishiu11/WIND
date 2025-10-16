# WIND vs. DIM: A Comparative Performance Benchmark Plan

## 1. Introduction

The WIND protocol has been developed as a modern, high-performance alternative to the established DIM protocol. This document outlines a comprehensive and rigorous benchmark plan to empirically evaluate and compare the performance characteristics of both protocols. The primary goal is to provide a detailed, data-driven analysis of their respective strengths and weaknesses across various real-world scenarios, with a focus on latency, throughput, and scalability.

This plan is structured into two primary suites of tests:
* **Suite A** focuses on **deterministic workloads** with fixed message sizes and constant transmission rates to establish a clear performance baseline under ideal, repeatable conditions.
* **Suite B** introduces **stochastic workloads**, using a Poisson process for message timing and a mixed distribution of message sizes to simulate the bursty and unpredictable nature of real-world traffic.

This dual approach ensures a thorough, state-of-the-art evaluation that is paper-worthy, can withstand peer review, and provides a solid foundation for any claims of performance improvements.

## 2. Goals and Objectives

The primary goal of this benchmark is to **quantify the performance differences between the WIND and DIM protocols**. This will be achieved through the following objectives:

* **Establish a baseline:** Measure and compare performance under controlled, ideal conditions (Suite A).
* **Assess real-world performance:** Evaluate how each protocol handles unpredictable, bursty traffic and mixed data loads (Suite B).
* **Latency Analysis:** Compare end-to-end message latency across all scenarios.
* **Throughput Analysis:** Determine the maximum sustainable message throughput.
* **Scalability Assessment:** Evaluate how performance changes with an increasing number of clients and data streams.
* **Resource Utilization:** Measure and compare the CPU and memory consumption of both protocols.

## 3. Benchmark Environment

To ensure the reproducibility and validity of the benchmark results, a well-defined and consistent environment is crucial.

### 3.1. Hardware *(to be finalized for official benchmarks)*

All tests will be conducted on dedicated, physical machines with the following specifications:

* **CPU:** Intel Xeon E-2288G @ 3.70GHz (8 cores, 16 threads)
* **RAM:** 64GB DDR4 ECC @ 2666MHz
* **Network:** 10 Gigabit Ethernet (Intel X550-T2)
* **Storage:** 1TB NVMe SSD

### 3.2. Software *(to be finalized for official benchmarks)*

* **Operating System:** Ubuntu 22.04 LTS (with the low-latency kernel)
* **Protocol Versions:**
    * **WIND:** Latest version from the `main` branch.
    * **DIM:** Latest stable release.
* **Benchmarking Tool:** The existing `wind-bench` crate will be extended into a comprehensive framework to ensure consistent measurement methodology and minimize overhead for both protocols.

### 3.3. Network Configuration *(to be finalized for official benchmarks)*

* All test machines will be connected to a dedicated 10 Gigabit Ethernet switch.
* Network latency between machines will be measured and documented before the tests.
* Network conditions will be controlled to ensure no external traffic interferes with the benchmark.

## 4. Methodology

### 4.1. Test Harness

A dedicated test harness will be developed to automate the benchmark scenarios. This harness will be responsible for:

* Deploying and configuring the protocol servers (WIND Registry and DIM Name Server).
* Starting and managing the publisher and subscriber clients.
* Implementing both deterministic and stochastic load generation models.
* Collecting and aggregating the performance metrics.
* Generating reports in a consistent format.

### 4.2. Measurement Techniques

* **Latency:** End-to-end latency will be measured by timestamping messages at the publisher and then subtracting the send time from the receive time at the subscriber. High-resolution clocks will be used, and clock synchronization between machines will be ensured using NTP.
* **Throughput:** Throughput will be measured by counting the number of messages successfully received by the subscribers over a fixed period.
* **Resource Utilization:** A system monitoring tool (e.g., `psutil` or `collectd`) will be used to track the CPU and memory usage of the server and client processes.

### 4.3. Statistical Analysis

* All tests will be run a minimum of 5 times to ensure statistical significance.
* The results will be presented with mean, median, standard deviation, and key percentile values (p50, p90, p95, p99, p99.9).
* Histograms and box plots will be used to visualize the distribution of the collected data, which is especially important for the stochastic workloads.

## 5. Suite A: Deterministic Workload Benchmarks

This suite establishes a performance baseline using constant-rate traffic and fixed message sizes. This helps identify the raw, best-case performance characteristics of each protocol.

### 5.1. Scenario A1: Baseline Latency (1 Publisher, 1 Subscriber)

* **Description:** A single publisher sends messages to a single subscriber.
* **Variables:**
    * **Message Size:** 64 bytes, 1KB, 16KB, 64KB
    * **Publishing Rate (Constant):** 1,000 msg/s, 10,000 msg/s, 100,000 msg/s
* **Metrics:**
    * End-to-end latency (p50, p99, max)
    * CPU and memory utilization

### 5.2. Scenario A2: Fan-Out Throughput (1 Publisher, N Subscribers)

* **Description:** A single publisher sends messages to an increasing number of subscribers.
* **Variables:**
    * **Number of Subscribers (N):** 1, 10, 50, 100, 500
    * **Message Size:** 1KB
    * **Publishing Rate (Constant):** 10,000 msg/s
* **Metrics:**
    * Aggregate throughput
    * End-to-end latency
    * Server CPU and memory utilization

### 5.3. Scenario A3: Fan-In Stress (N Publishers, 1 Subscriber)

* **Description:** An increasing number of publishers send messages to a single subscriber.
* **Variables:**
    * **Number of Publishers (N):** 1, 10, 50, 100
    * **Message Size:** 1KB
    * **Publishing Rate (Constant, per publisher):** 1,000 msg/s
* **Metrics:**
    * End-to-end latency
    * Message loss rate
    * Subscriber CPU and memory utilization

### 5.4. Scenario A4: Scalability (N Publishers, M Subscribers)

* **Description:** A large number of publishers send messages to a large number of subscribers, with each subscriber connected to a subset of the publishers. This scenario tests the overall scalability of the protocols.
* **Variables:**
    * **(N, M):** (10, 100), (50, 500), (100, 1000)
    * **Message Size:** 1KB
    * **Publishing Rate (per publisher):** 1,000 msg/s
* **Metrics:**
    * Aggregate throughput
    * End-to-end latency
    * CPU and memory utilization of the servers and clients

## 6. Suite B: Stochastic Workload Benchmarks

This suite simulates a more realistic production environment with unpredictable traffic patterns and variable data sizes.

### 6.1. Workload Modeling

* **Message Inter-Arrival Time:** To simulate bursty traffic, message publication will be modeled as a **Poisson process**. The configured publishing rate (e.g., 10,000 msg/s) will serve as the mean arrival rate (**λ**). The time between individual message publications will follow an **Exponential distribution** with a mean of `1/λ`.
* **Message Size Distribution:** To simulate varied data, payloads will follow a **multimodal distribution** defined as the "Typical IoT" mix:
    * **70%** small messages (randomly sized between 64-256 bytes)
    * **25%** medium messages (randomly sized between 1KB-4KB)
    * **5%** large messages (randomly sized between 16KB-32KB)
* **Message Content:** Payloads will be generated with pseudo-random, non-compressible data using a fixed seed for each run to ensure fair, reproducible comparisons.

### 6.2. Scenario B1: Real-World Latency Profile (1 Publisher, 1 Subscriber)

* **Description:** Measures the latency distribution under bursty traffic and mixed payload sizes.
* **Variables:**
    * **Mean Publishing Rate (λ):** 1,000 msg/s, 10,000 msg/s, 100,000 msg/s
    * **Message Size Profile:** "Typical IoT" mix
* **Metrics:**
    * Full latency histogram analysis (p50, p90, p95, p99, p99.9, max)
    * CPU and memory utilization

### 6.3. Scenario B2: Scalability Under Chaos (N Publishers, M Subscribers)

* **Description:** A comprehensive stress test combining fan-in and fan-out with a realistic workload.
* **Variables:**
    * **Topology (N Publishers, M Subscribers):** (10, 100), (50, 500), (100, 1000)
    * **Mean Publishing Rate (λ, per publisher):** 1,000 msg/s
    * **Message Size Profile:** "Typical IoT" mix
* **Metrics:**
    * Aggregate throughput
    * End-to-end latency distribution
    * Message loss rate
    * CPU and memory utilization of servers and clients

## 7. Data Analysis and Reporting

The results of both suites will be compiled into a single, comprehensive report.

* **Executive Summary:** A high-level overview of the key findings, contrasting the results from Suite A and Suite B.
* **Benchmark Setup:** A detailed description of the final hardware, software, and network configuration.
* **Results:** A detailed presentation of the collected data for each scenario, including tables, charts, and graphs. Analysis will compare the deterministic vs. stochastic results to highlight how each protocol's performance changes under pressure.
* **Analysis:** A thorough analysis of the results, explaining the performance differences observed. For example, "While Protocol X showed 10% lower latency in the deterministic test (A1), its p99 latency increased by 50% under the stochastic load (B1), suggesting issues with handling traffic bursts."
* **Conclusion:** A summary of the findings and data-driven recommendations for when to use each protocol.

## 8. Ethical Considerations

This benchmark will be conducted with the utmost fairness and transparency.

* **Reproducibility:** The benchmark code, configuration, and raw data will be made publicly available.
* **Fair Comparison:** Both protocols will be configured for optimal performance, following the best practices recommended by their respective developers.
* **Objectivity:** The results will be presented in an unbiased manner, without any preconceived notions about the outcome.
