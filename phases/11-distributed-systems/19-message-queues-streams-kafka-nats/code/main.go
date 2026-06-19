package main

import (
	"fmt"
	"hash/fnv"
	"sort"
	"strings"
	"sync"
	"time"
)

type Message struct {
	Key       string
	Value     []byte
	Timestamp time.Time
	Offset    int64
	Partition int
}

type RetentionPolicy struct {
	MaxMessages int64
	MaxAgeMs    int64
}

type Partition struct {
	mu         sync.RWMutex
	id         int
	messages   []Message
	nextOffset int64
}

func (p *Partition) Append(key string, value []byte) Message {
	p.mu.Lock()
	defer p.mu.Unlock()
	msg := Message{
		Key:       key,
		Value:     value,
		Timestamp: time.Now(),
		Offset:    p.nextOffset,
		Partition: p.id,
	}
	p.nextOffset++
	p.messages = append(p.messages, msg)
	return msg
}

func (p *Partition) Read(fromOffset int64, maxCount int) []Message {
	p.mu.RLock()
	defer p.mu.RUnlock()
	startIdx := -1
	for i, m := range p.messages {
		if m.Offset >= fromOffset {
			startIdx = i
			break
		}
	}
	if startIdx == -1 {
		return nil
	}
	endIdx := startIdx + maxCount
	if endIdx > len(p.messages) {
		endIdx = len(p.messages)
	}
	result := make([]Message, endIdx-startIdx)
	copy(result, p.messages[startIdx:endIdx])
	return result
}

func (p *Partition) Stats() (count int, earliest, latest int64) {
	p.mu.RLock()
	defer p.mu.RUnlock()
	count = len(p.messages)
	if count == 0 {
		return count, 0, -1
	}
	earliest = p.messages[0].Offset
	latest = p.messages[count-1].Offset
	return
}

type Topic struct {
	Name       string
	Partitions []*Partition
	Retention  RetentionPolicy
}

type ConsumerState struct {
	ID               string
	CommittedOffsets map[int]int64
}

type ConsumerGroup struct {
	GroupID     string
	TopicName   string
	mu          sync.RWMutex
	consumers   map[string]*ConsumerState
	assignments map[string][]int
}

func (g *ConsumerGroup) rebalance(numPartitions int) {
	ids := make([]string, 0, len(g.consumers))
	for id := range g.consumers {
		ids = append(ids, id)
	}
	sort.Strings(ids)

	g.assignments = make(map[string][]int, len(ids))
	for _, id := range ids {
		g.assignments[id] = nil
	}
	for p := 0; p < numPartitions; p++ {
		cid := ids[p%len(ids)]
		g.assignments[cid] = append(g.assignments[cid], p)
	}
}

type Subscription struct {
	GroupID    string
	ConsumerID string
	TopicName  string
}

type Broker struct {
	mu     sync.RWMutex
	topics map[string]*Topic
	groups map[string]*ConsumerGroup
}

func NewBroker() *Broker {
	return &Broker{
		topics: make(map[string]*Topic),
		groups: make(map[string]*ConsumerGroup),
	}
}

func (b *Broker) CreateTopic(name string, numPartitions int, retention RetentionPolicy) *Topic {
	b.mu.Lock()
	defer b.mu.Unlock()
	if t, exists := b.topics[name]; exists {
		return t
	}
	partitions := make([]*Partition, numPartitions)
	for i := range partitions {
		partitions[i] = &Partition{id: i, messages: make([]Message, 0), nextOffset: 0}
	}
	t := &Topic{Name: name, Partitions: partitions, Retention: retention}
	b.topics[name] = t
	return t
}

func hashKey(key string, numPartitions int) int {
	if key == "" {
		return 0
	}
	h := fnv.New32a()
	h.Write([]byte(key))
	return int(h.Sum32() % uint32(numPartitions))
}

func (b *Broker) Publish(topicName, key string, value []byte) (Message, error) {
	b.mu.RLock()
	topic, exists := b.topics[topicName]
	b.mu.RUnlock()
	if !exists {
		return Message{}, fmt.Errorf("topic %q not found", topicName)
	}
	partIdx := hashKey(key, len(topic.Partitions))
	msg := topic.Partitions[partIdx].Append(key, value)
	return msg, nil
}

func (b *Broker) getOrCreateGroup(groupID, topicName string) (*ConsumerGroup, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if g, ok := b.groups[groupID]; ok {
		return g, nil
	}
	if _, ok := b.topics[topicName]; !ok {
		return nil, fmt.Errorf("topic %q not found", topicName)
	}
	g := &ConsumerGroup{
		GroupID:     groupID,
		TopicName:   topicName,
		consumers:   make(map[string]*ConsumerState),
		assignments: make(map[string][]int),
	}
	b.groups[groupID] = g
	return g, nil
}

func (b *Broker) Subscribe(topicName, groupID, consumerID string) (*Subscription, error) {
	b.mu.RLock()
	topic, exists := b.topics[topicName]
	b.mu.RUnlock()
	if !exists {
		return nil, fmt.Errorf("topic %q not found", topicName)
	}

	g, err := b.getOrCreateGroup(groupID, topicName)
	if err != nil {
		return nil, err
	}

	g.mu.Lock()
	defer g.mu.Unlock()

	cs := &ConsumerState{
		ID:               consumerID,
		CommittedOffsets: make(map[int]int64),
	}
	for p := 0; p < len(topic.Partitions); p++ {
		cs.CommittedOffsets[p] = 0
	}
	g.consumers[consumerID] = cs
	g.rebalance(len(topic.Partitions))

	return &Subscription{GroupID: groupID, ConsumerID: consumerID, TopicName: topicName}, nil
}

func (b *Broker) Consume(sub *Subscription, maxPerPartition int) ([]Message, error) {
	b.mu.RLock()
	topic, exists := b.topics[sub.TopicName]
	b.mu.RUnlock()
	if !exists {
		return nil, fmt.Errorf("topic %q not found", sub.TopicName)
	}

	g, exists := b.groups[sub.GroupID]
	if !exists {
		return nil, fmt.Errorf("group %q not found", sub.GroupID)
	}

	g.mu.RLock()
	consumer, ok := g.consumers[sub.ConsumerID]
	if !ok {
		g.mu.RUnlock()
		return nil, fmt.Errorf("consumer %q not found", sub.ConsumerID)
	}
	assigned := make([]int, len(g.assignments[sub.ConsumerID]))
	copy(assigned, g.assignments[sub.ConsumerID])
	g.mu.RUnlock()

	var results []Message
	for _, partIdx := range assigned {
		fromOffset := consumer.CommittedOffsets[partIdx]
		msgs := topic.Partitions[partIdx].Read(fromOffset, maxPerPartition)
		results = append(results, msgs...)
	}
	return results, nil
}

func (b *Broker) CommitOffset(sub *Subscription, partition int, offset int64) error {
	g, exists := b.groups[sub.GroupID]
	if !exists {
		return fmt.Errorf("group %q not found", sub.GroupID)
	}
	g.mu.Lock()
	defer g.mu.Unlock()
	consumer, exists := g.consumers[sub.ConsumerID]
	if !exists {
		return fmt.Errorf("consumer %q not found", sub.ConsumerID)
	}
	consumer.CommittedOffsets[partition] = offset + 1
	return nil
}

func (b *Broker) RemoveConsumer(groupID, consumerID string) error {
	g, exists := b.groups[groupID]
	if !exists {
		return fmt.Errorf("group %q not found", groupID)
	}
	b.mu.RLock()
	topic, exists := b.topics[g.TopicName]
	b.mu.RUnlock()
	if !exists {
		return fmt.Errorf("topic %q not found", g.TopicName)
	}

	g.mu.Lock()
	defer g.mu.Unlock()
	delete(g.consumers, consumerID)
	g.rebalance(len(topic.Partitions))
	return nil
}

func (b *Broker) GetAssignments(groupID string) map[string][]int {
	g := b.groups[groupID]
	g.mu.RLock()
	defer g.mu.RUnlock()
	result := make(map[string][]int, len(g.assignments))
	for k, v := range g.assignments {
		cp := make([]int, len(v))
		copy(cp, v)
		result[k] = cp
	}
	return result
}

func printHeader(title string) {
	fmt.Println()
	fmt.Println(strings.Repeat("=", 70))
	fmt.Printf("  %s\n", title)
	fmt.Println(strings.Repeat("=", 70))
}

func printSection(title string) {
	fmt.Println()
	fmt.Printf("--- %s ---\n", title)
}

func main() {
	broker := NewBroker()

	printHeader("LESSON 19: Message Queues & Streams — Kafka, NATS")

	printSection("1. Creating topic 'orders' with 4 partitions")
	topic := broker.CreateTopic("orders", 4, RetentionPolicy{MaxMessages: 10000, MaxAgeMs: 86400000})
	fmt.Printf("Topic 'orders' created with %d partitions\n", len(topic.Partitions))

	printSection("2. Publishing 100 messages with key-based partitioning")
	keys := []string{"user-alice", "user-bob", "user-carol", "user-dave", "user-eve"}
	partitionCounts := make(map[int]int)

	for i := 0; i < 100; i++ {
		key := keys[i%len(keys)]
		value := []byte(fmt.Sprintf("order-%03d", i))
		msg, _ := broker.Publish("orders", key, value)
		partitionCounts[msg.Partition]++
		if i < 5 || i == 99 {
			fmt.Printf("  msg %3d: key=%-12s partition=%d offset=%d\n", i, key, msg.Partition, msg.Offset)
		} else if i == 5 {
			fmt.Println("  ... (messages 5-98 omitted)")
		}
	}

	fmt.Println()
	fmt.Println("Partition distribution (key-based hashing):")
	for p := 0; p < 4; p++ {
		fmt.Printf("  Partition %d: %d messages\n", p, partitionCounts[p])
	}

	fmt.Println()
	fmt.Println("Key ordering guarantee:")
	for _, key := range keys[:3] {
		pIdx := hashKey(key, 4)
		fmt.Printf("  key=%-12s always maps to partition %d\n", key, pIdx)
	}
	fmt.Println("  → Per-key FIFO ordering is guaranteed within each partition")

	printSection("3. Consumer Group 'analytics' — 2 consumers, 4 partitions")
	subA, _ := broker.Subscribe("orders", "analytics", "consumer-A")
	subB, _ := broker.Subscribe("orders", "analytics", "consumer-B")

	assignments := broker.GetAssignments("analytics")
	for cid, parts := range assignments {
		fmt.Printf("  %s → partitions %v\n", cid, parts)
	}

	msgsA, _ := broker.Consume(subA, 100)
	msgsB, _ := broker.Consume(subB, 100)
	fmt.Println()
	fmt.Printf("  consumer-A: %d messages from partitions %v\n", len(msgsA), assignments["consumer-A"])
	fmt.Printf("  consumer-B: %d messages from partitions %v\n", len(msgsB), assignments["consumer-B"])
	fmt.Printf("  total consumed: %d (should equal 100)\n", len(msgsA)+len(msgsB))

	if len(msgsA) > 3 {
		fmt.Println()
		fmt.Println("  First 3 messages consumed by consumer-A:")
		for _, m := range msgsA[:3] {
			fmt.Printf("    partition=%d offset=%-3d key=%-12s value=%s\n", m.Partition, m.Offset, m.Key, string(m.Value))
		}
	}

	for _, partIdx := range assignments["consumer-A"] {
		var maxOff int64
		for _, m := range msgsA {
			if m.Partition == partIdx && m.Offset > maxOff {
				maxOff = m.Offset
			}
		}
		if maxOff > 0 || len(msgsA) > 0 {
			broker.CommitOffset(subA, partIdx, maxOff)
		}
	}
	for _, partIdx := range assignments["consumer-B"] {
		var maxOff int64
		for _, m := range msgsB {
			if m.Partition == partIdx && m.Offset > maxOff {
				maxOff = m.Offset
			}
		}
		if maxOff > 0 || len(msgsB) > 0 {
			broker.CommitOffset(subB, partIdx, maxOff)
		}
	}
	fmt.Println()
	fmt.Println("  Offset tracking: both consumers committed their latest offsets")

	printSection("4. Second consumer group 'audit' — independent consumption")
	subC, _ := broker.Subscribe("orders", "audit", "consumer-C")
	auditAssign := broker.GetAssignments("audit")
	fmt.Printf("  consumer-C (audit) → partitions %v\n", auditAssign["consumer-C"])
	msgsC, _ := broker.Consume(subC, 100)
	fmt.Printf("  consumer-C received %d messages (independent from analytics group)\n", len(msgsC))

	printSection("5. Consumer failure → partition rebalancing")
	fmt.Println("  Before: 2 consumers in 'analytics' group")
	for cid, parts := range broker.GetAssignments("analytics") {
		fmt.Printf("    %s → partitions %v\n", cid, parts)
	}

	fmt.Println()
	fmt.Println("  *** consumer-B crashes! ***")
	_ = broker.RemoveConsumer("analytics", "consumer-B")

	fmt.Println()
	fmt.Println("  After rebalancing:")
	for cid, parts := range broker.GetAssignments("analytics") {
		fmt.Printf("    %s → partitions %v\n", cid, parts)
	}

	fmt.Println()
	fmt.Println("  consumer-A now consumes from ALL 4 partitions")
	fmt.Println("  (Previously: 2 partitions; now: 4 partitions)")

	msgsRecovery, _ := broker.Consume(subA, 100)
	fmt.Printf("  consumer-A reads %d remaining messages across all partitions\n", len(msgsRecovery))

	printSection("6. Point-to-Point vs Pub/Sub")
	fmt.Println("  Point-to-Point (Queue):")
	fmt.Println("    One message → exactly one consumer in the group")
	fmt.Println("    Use for: task distribution, work queues")
	fmt.Println()
	fmt.Println("  Pub/Sub (Topic):")
	fmt.Println("    One message → every subscriber receives it")
	fmt.Println("    Use for: event notification, fan-out")
	fmt.Println()
	fmt.Println("  Our broker implements point-to-point via consumer groups:")
	fmt.Println("    Each partition is consumed by exactly one member of the group")
	fmt.Println("    Different groups consume independently (pub/sub across groups)")

	printSection("7. Delivery Semantics")
	fmt.Println("  At-most-once:")
	fmt.Println("    Producer: fire and forget (acks=0)")
	fmt.Println("    Consumer: commit offset before processing")
	fmt.Println("    → Messages can be lost, no duplicates")
	fmt.Println("    → Use for: telemetry, metrics (losing a point is OK)")
	fmt.Println()
	fmt.Println("  At-least-once:")
	fmt.Println("    Producer: wait for all replicas (acks=all)")
	fmt.Println("    Consumer: commit offset AFTER processing")
	fmt.Println("    → No data loss, but duplicates on consumer crash")
	fmt.Println("    → You MUST make consumers idempotent")
	fmt.Println("    → Use for: orders, events (most common default)")
	fmt.Println()
	fmt.Println("  Exactly-once:")
	fmt.Println("    Idempotent producer + transactional consume-process-write")
	fmt.Println("    → No data loss, no duplicates, but high overhead")
	fmt.Println("    → Use for: financial systems, payment processing")

	printSection("8. NATS Subject-Based Addressing")
	fmt.Println("  Kafka:    topic = 'orders' (flat, partitioned)")
	fmt.Println("  NATS:     subject = 'orders.created.electronics' (hierarchical)")
	fmt.Println()
	fmt.Println("  NATS wildcards:")
	fmt.Println("    * matches one token:   orders.*.electronics → orders.created.electronics")
	fmt.Println("    > matches all tokens:   orders.> → orders.created, orders.created.electronics, ...")
	fmt.Println()
	fmt.Println("  Core NATS:  at-most-once, fire-and-forget, ~sub-ms latency")
	fmt.Println("  JetStream:  at-least-once / exactly-once, durable, replayable (like Kafka)")

	printSection("9. Partition Internals")
	for i, p := range topic.Partitions {
		count, earliest, latest := p.Stats()
		fmt.Printf("  Partition %d: %d messages, offsets [%d .. %d]\n", i, count, earliest, latest)
	}

	printHeader("Summary")
	fmt.Println("  Message queues decouple producers from consumers.")
	fmt.Println("  Partitions provide parallelism and per-key ordering.")
	fmt.Println("  Consumer groups provide horizontal scalability.")
	fmt.Println("  Rebalancing provides fault tolerance on consumer failure.")
	fmt.Println("  Choose delivery semantics by your tolerance for duplicates vs cost.")
	fmt.Println(strings.Repeat("=", 70))
}