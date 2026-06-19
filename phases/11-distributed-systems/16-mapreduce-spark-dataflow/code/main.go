package main

import (
	"crypto/md5"
	"encoding/hex"
	"fmt"
	"regexp"
	"sort"
	"strings"
)

type KeyValue struct {
	Key   string
	Value interface{}
}

type MapFunc func(key string, value string) []KeyValue
type ReduceFunc func(key string, values []interface{}) interface{}
type CombineFunc func(key string, values []interface{}) interface{}

func partitionKey(key string, numReducers int) int {
	h := md5.Sum([]byte(key))
	hexStr := hex.EncodeToString(h[:])
	var hash uint
	fmt.Sscanf(hexStr[:8], "%x", &hash)
	return int(hash % uint(numReducers))
}

type Mapper struct {
	mapFn MapFunc
}

func (m *Mapper) Run(chunk []string) []KeyValue {
	var results []KeyValue
	for _, line := range chunk {
		if strings.TrimSpace(line) == "" {
			continue
		}
		results = append(results, m.mapFn("", line)...)
	}
	return results
}

type Shuffle struct {
	numReducers int
}

func (s *Shuffle) Partition(pairs []KeyValue) map[int][]KeyValue {
	buckets := make(map[int][]KeyValue)
	for i := 0; i < s.numReducers; i++ {
		buckets[i] = []KeyValue{}
	}
	for _, kv := range pairs {
		bucket := partitionKey(kv.Key, s.numReducers)
		buckets[bucket] = append(buckets[bucket], kv)
	}
	return buckets
}

func (s *Shuffle) Group(buckets map[int][]KeyValue) map[int]map[string][]interface{} {
	grouped := make(map[int]map[string][]interface{})
	for bucketID, pairs := range buckets {
		g := make(map[string][]interface{})
		for _, kv := range pairs {
			g[kv.Key] = append(g[kv.Key], kv.Value)
		}
		grouped[bucketID] = g
	}
	return grouped
}

type Combiner struct {
	combineFn CombineFunc
}

func (c *Combiner) Run(pairs []KeyValue) []KeyValue {
	if c.combineFn == nil {
		return pairs
	}
	grouped := make(map[string][]interface{})
	var order []string
	for _, kv := range pairs {
		if _, exists := grouped[kv.Key]; !exists {
			order = append(order, kv.Key)
		}
		grouped[kv.Key] = append(grouped[kv.Key], kv.Value)
	}
	var results []KeyValue
	for _, key := range order {
		results = append(results, KeyValue{Key: key, Value: c.combineFn(key, grouped[key])})
	}
	return results
}

type Reducer struct {
	reduceFn ReduceFunc
}

func (r *Reducer) Run(grouped map[string][]interface{}) []KeyValue {
	var keys []string
	for k := range grouped {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	var results []KeyValue
	for _, key := range keys {
		results = append(results, KeyValue{Key: key, Value: r.reduceFn(key, grouped[key])})
	}
	return results
}

type JobStats struct {
	TotalMapPairs     int
	PostCombinerPairs int
	ShufflePairs      int
}

type MapReduceJob struct {
	mapFn       MapFunc
	reduceFn    ReduceFunc
	combineFn   CombineFunc
	numMappers  int
	numReducers int
}

func NewMapReduceJob(mapFn MapFunc, reduceFn ReduceFunc, numMappers, numReducers int, combineFn CombineFunc) *MapReduceJob {
	return &MapReduceJob{
		mapFn:       mapFn,
		reduceFn:    reduceFn,
		combineFn:   combineFn,
		numMappers:  numMappers,
		numReducers: numReducers,
	}
}

func (j *MapReduceJob) splitInput(data string) [][]string {
	lines := strings.Split(data, "\n")
	chunkSize := len(lines) / j.numMappers
	if len(lines)%j.numMappers != 0 {
		chunkSize++
	}
	if chunkSize < 1 {
		chunkSize = 1
	}

	var chunks [][]string
	for i := 0; i < len(lines); i += chunkSize {
		end := i + chunkSize
		if end > len(lines) {
			end = len(lines)
		}
		chunks = append(chunks, lines[i:end])
	}
	for len(chunks) < j.numMappers {
		chunks = append(chunks, []string{})
	}
	return chunks
}

func (j *MapReduceJob) Run(data string) []KeyValue {
	mapper := &Mapper{mapFn: j.mapFn}
	shuffle := &Shuffle{numReducers: j.numReducers}
	combiner := &Combiner{combineFn: j.combineFn}
	reducer := &Reducer{reduceFn: j.reduceFn}

	chunks := j.splitInput(data)

	var allIntermediate []KeyValue
	for _, chunk := range chunks {
		pairs := mapper.Run(chunk)
		if j.combineFn != nil {
			pairs = combiner.Run(pairs)
		}
		allIntermediate = append(allIntermediate, pairs...)
	}

	buckets := shuffle.Partition(allIntermediate)
	grouped := shuffle.Group(buckets)

	var results []KeyValue
	for bucketID := 0; bucketID < j.numReducers; bucketID++ {
		if g, ok := grouped[bucketID]; ok {
			results = append(results, reducer.Run(g)...)
		}
	}

	sort.Slice(results, func(i, j int) bool {
		return results[i].Key < results[j].Key
	})
	return results
}

func (j *MapReduceJob) RunWithFailure(data string, failMapperIndex int) ([]KeyValue, JobStats) {
	mapper := &Mapper{mapFn: j.mapFn}
	shuffle := &Shuffle{numReducers: j.numReducers}
	combiner := &Combiner{combineFn: j.combineFn}
	reducer := &Reducer{reduceFn: j.reduceFn}

	chunks := j.splitInput(data)
	var stats JobStats

	var allIntermediate []KeyValue
	for i, chunk := range chunks {
		pairs := mapper.Run(chunk)
		stats.TotalMapPairs += len(pairs)

		if i == failMapperIndex {
			pairs = mapper.Run(chunk)
		}

		if j.combineFn != nil {
			pairs = combiner.Run(pairs)
			stats.PostCombinerPairs += len(pairs)
		}
		stats.ShufflePairs += len(pairs)
		allIntermediate = append(allIntermediate, pairs...)
	}

	buckets := shuffle.Partition(allIntermediate)
	grouped := shuffle.Group(buckets)

	var results []KeyValue
	for bucketID := 0; bucketID < j.numReducers; bucketID++ {
		if g, ok := grouped[bucketID]; ok {
			results = append(results, reducer.Run(g)...)
		}
	}

	sort.Slice(results, func(i, j int) bool {
		return results[i].Key < results[j].Key
	})
	return results, stats
}

var wordRe = regexp.MustCompile(`[a-zA-Z]+`)

func wordCountMap(_ string, line string) []KeyValue {
	var pairs []KeyValue
	words := wordRe.FindAllString(strings.ToLower(line), -1)
	for _, w := range words {
		pairs = append(pairs, KeyValue{Key: w, Value: 1})
	}
	return pairs
}

func wordCountReduce(_ string, values []interface{}) interface{} {
	sum := 0
	for _, v := range values {
		sum += v.(int)
	}
	return sum
}

func invertedIndexMap(_ string, line string) []KeyValue {
	parts := strings.SplitN(strings.TrimSpace(line), "\t", 2)
	if len(parts) != 2 {
		return nil
	}
	docID, text := parts[0], parts[1]
	var pairs []KeyValue
	words := wordRe.FindAllString(strings.ToLower(text), -1)
	for _, w := range words {
		pairs = append(pairs, KeyValue{Key: w, Value: docID})
	}
	return pairs
}

func invertedIndexReduce(_ string, values []interface{}) interface{} {
	seen := make(map[string]bool)
	var result []string
	for _, v := range values {
		docID := v.(string)
		if !seen[docID] {
			seen[docID] = true
			result = append(result, docID)
		}
	}
	sort.Strings(result)
	return result
}

func section(title string) {
	fmt.Printf("\n%s\n", strings.Repeat("=", 65))
	fmt.Printf("  %s\n", title)
	fmt.Printf("%s\n", strings.Repeat("=", 65))
}

func main() {
	section("Word Count — Basic MapReduce")
	text := "the quick brown fox jumps over the lazy dog\n" +
		"the fox was quick and the dog was lazy\n" +
		"a quick brown fox and a lazy dog"

	job := NewMapReduceJob(wordCountMap, wordCountReduce, 2, 2, nil)
	results := job.Run(text)
	fmt.Println("Input:")
	fmt.Println(text)
	fmt.Println("\nWord counts:")
	for _, kv := range results {
		fmt.Printf("  %s: %v\n", kv.Key, kv.Value)
	}

	section("Word Count — With Combiner")
	jobNoCombiner := NewMapReduceJob(wordCountMap, wordCountReduce, 2, 2, nil)
	_, statsNo := jobNoCombiner.RunWithFailure(text, -1)

	jobCombiner := NewMapReduceJob(wordCountMap, wordCountReduce, 2, 2, wordCountReduce)
	_, statsYes := jobCombiner.RunWithFailure(text, -1)

	fmt.Printf("Without combiner: %d pairs shuffled\n", statsNo.ShufflePairs)
	fmt.Printf("With combiner:    %d pairs shuffled\n", statsYes.ShufflePairs)
	reduction := statsNo.ShufflePairs - statsYes.ShufflePairs
	percent := 100 * reduction / statsNo.ShufflePairs
	fmt.Printf("Reduction:        %d pairs saved (%d%%)\n", reduction, percent)

	section("Fault Tolerance — Mapper Re-execution")
	resultsNormal, _ := jobNoCombiner.RunWithFailure(text, -1)
	resultsFailed, _ := jobNoCombiner.RunWithFailure(text, 0)

	fmt.Println("Normal execution:")
	for _, kv := range resultsNormal {
		fmt.Printf("  %s: %v\n", kv.Key, kv.Value)
	}
	fmt.Println("\nWith mapper-0 failure and re-execution:")
	for _, kv := range resultsFailed {
		fmt.Printf("  %s: %v\n", kv.Key, kv.Value)
	}

	match := true
	if len(resultsNormal) != len(resultsFailed) {
		match = false
	} else {
		for i := range resultsNormal {
			if resultsNormal[i].Key != resultsFailed[i].Key || resultsNormal[i].Value != resultsFailed[i].Value {
				match = false
			}
		}
	}
	if match {
		fmt.Println("\nResults match: deterministic mapper re-execution produces identical output.")
	} else {
		fmt.Println("\nResults differ!")
	}

	section("Inverted Index MapReduce")
	documents := "doc1\tthe quick brown fox\n" +
		"doc2\tthe lazy dog sleeps\n" +
		"doc3\tthe quick fox and the dog"

	idxJob := NewMapReduceJob(invertedIndexMap, invertedIndexReduce, 2, 2, nil)
	idxResults := idxJob.Run(documents)

	fmt.Println("Input documents:")
	for _, line := range strings.Split(documents, "\n") {
		fmt.Printf("  %s\n", line)
	}
	fmt.Println("\nInverted index:")
	for _, kv := range idxResults {
		fmt.Printf("  %s: %v\n", kv.Key, kv.Value)
	}

	section("Partitioning — Hash-Based Key Distribution")
	keys := []string{"apple", "banana", "cherry", "date", "elderberry", "fig", "grape"}
	numReducers := 3
	buckets := make(map[int][]string)
	for _, key := range keys {
		b := partitionKey(key, numReducers)
		buckets[b] = append(buckets[b], key)
	}
	fmt.Printf("Distributing %d keys across %d reducers:\n", len(keys), numReducers)
	for i := 0; i < numReducers; i++ {
		fmt.Printf("  Reducer %d: %v\n", i, buckets[i])
	}

	section("System Comparison")
	fmt.Printf("%-25s %-20s %-20s %-20s\n", "Property", "MapReduce", "Spark", "Dataflow/Beam")
	fmt.Println(strings.Repeat("-", 85))
	rows := []struct {
		prop, mr, spark, df string
	}{
		{"Model", "Batch only", "Batch + iterative", "Batch + streaming"},
		{"Intermediate data", "Disk", "Memory (lineage)", "Memory (windowed)"},
		{"Iterative workloads", "Poor (I/O/iter)", "Excellent (cache)", "Good (state)"},
		{"Fault tolerance", "Re-execute tasks", "Lineage recompute", "Checkpoint+replay"},
		{"Latency", "Minutes", "Seconds", "ms to seconds"},
		{"Streaming", "No", "Micro-batches", "True streaming"},
		{"Programming model", "Map + Reduce", "Transform + Action", "ParDo+GBK+Window"},
	}
	for _, r := range rows {
		fmt.Printf("%-25s %-20s %-20s %-20s\n", r.prop, r.mr, r.spark, r.df)
	}
}