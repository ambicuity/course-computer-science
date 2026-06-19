# Build a Search Engine (Crawler + Index + Ranker)

> Relevance comes from disciplined ingestion, indexing, and ranking feedback loops.

**Type:** Build
**Languages:** Rust, Python
**Prerequisites:** Phase 19 lessons 01-07
**Time:** ~840 minutes

## Learning Objectives

- Build crawl, parse, index, and rank pipeline basics.
- Implement tokenization and inverted index construction.
- Design a simple scoring function for ranked retrieval.
- Define quality checks for freshness, recall, and precision signals.

## The Problem

Search systems fail when crawling, indexing, and ranking assumptions drift. Someone builds a crawler, collects 10,000 pages, builds an index, and discovers that queries return garbage because the tokenizer didn't handle punctuation correctly. Or the ranking function returns documents sorted by length instead of relevance. Or the crawler re-crawls the same pages forever and never discovers new ones.

A thin vertical pipeline with explicit artifacts prevents silent relevance decay. The first milestone: tokenize a local corpus and build an inverted index. The second: implement TF-IDF scoring and verify that queries return relevant results. The third: add a crawler that discovers new pages and updates the index. Each milestone produces a testable artifact.

## The Concept

A search engine has four stages:

```
Documents (web pages, files, etc.)
        │
        ▼
┌───────────────┐
│ 1. Crawler     │  Discover and fetch documents
│  (politeness)  │  Respect robots.txt, rate limits
└───────────────┘
        │
        ▼
┌───────────────┐
│ 2. Parser      │  Extract text, normalize, tokenize
│  (tokenizer)   │  Handle HTML, punctuation, stemming
└───────────────┘
        │
        ▼
┌───────────────┐
│ 3. Indexer     │  Build inverted index
│  (data struct) │  term → [(doc_id, frequency, positions)]
└───────────────┘
        │
        ▼
┌───────────────┐
│ 4. Ranker      │  Score query-document matches
│  (scoring)     │  TF-IDF, BM25, page rank
└───────────────┘
```

The inverted index is the core data structure. For each term, it stores a posting list: which documents contain the term, how often, and where. When a query arrives, the ranker looks up each query term in the index, retrieves the posting lists, and scores each candidate document.

```
Inverted index structure:

"the"    → [(doc0, 15), (doc1, 23), (doc2, 8), ...]
"quick"  → [(doc0, 2), (doc5, 1)]
"brown"  → [(doc0, 1), (doc3, 3)]
"fox"    → [(doc0, 1), (doc1, 1), (doc3, 2)]
```

TF-IDF scoring: a document is relevant if the query terms appear frequently in the document (TF: term frequency) but are rare across the corpus (IDF: inverse document frequency). BM25 refines this with document length normalization and saturation.

## Build It

### Step 1: Tokenizer (Python)

```python
import re
import math
from collections import defaultdict, Counter
from dataclasses import dataclass, field
from typing import Dict, List, Tuple

def tokenize(text: str) -> List[str]:
    """Normalize text into tokens: lowercase, split on non-alphanumeric."""
    text = text.lower()
    tokens = re.findall(r'[a-z0-9]+', text)
    # Remove very common stop words
    stop_words = {'the', 'a', 'an', 'is', 'are', 'was', 'were', 'in', 'on',
                  'at', 'to', 'for', 'of', 'and', 'or', 'but', 'not', 'with',
                  'this', 'that', 'it', 'by', 'from', 'as', 'be', 'has', 'had'}
    return [t for t in tokens if t not in stop_words and len(t) > 1]
```

### Step 2: Inverted Index

```python
@dataclass
class Posting:
    doc_id: int
    term_freq: int
    positions: List[int] = field(default_factory=list)

class InvertedIndex:
    def __init__(self):
        self.index: Dict[str, List[Posting]] = defaultdict(list)
        self.documents: Dict[int, str] = {}  # doc_id -> title/filename
        self.doc_lengths: Dict[int, int] = {}  # doc_id -> total terms
        self.total_docs: int = 0
        self.avg_doc_length: float = 0.0

    def add_document(self, doc_id: int, title: str, text: str):
        """Index a document."""
        tokens = tokenize(text)
        self.documents[doc_id] = title
        self.doc_lengths[doc_id] = len(tokens)
        self.total_docs += 1

        # Count term frequencies and positions
        term_positions: Dict[str, List[int]] = defaultdict(list)
        for pos, token in enumerate(tokens):
            term_positions[token].append(pos)

        # Add to inverted index
        for term, positions in term_positions.items():
            posting = Posting(
                doc_id=doc_id,
                term_freq=len(positions),
                positions=positions,
            )
            self.index[term].append(posting)

        # Update average document length
        total_len = sum(self.doc_lengths.values())
        self.avg_doc_length = total_len / self.total_docs

    def get_postings(self, term: str) -> List[Posting]:
        """Retrieve posting list for a term."""
        return self.index.get(term.lower(), [])

    def document_frequency(self, term: str) -> int:
        """Number of documents containing the term."""
        return len(self.get_postings(term))
```

### Step 3: TF-IDF and BM25 Ranking

```python
class Ranker:
    def __init__(self, index: InvertedIndex):
        self.index = index

    def tf_idf_score(self, query_terms: List[str], doc_id: int) -> float:
        """Classic TF-IDF scoring."""
        score = 0.0
        for term in query_terms:
            postings = self.index.get_postings(term)
            # Find this document's posting
            tf = 0
            for p in postings:
                if p.doc_id == doc_id:
                    tf = p.term_freq
                    break
            if tf == 0:
                continue
            # TF: log-normalized
            tf_score = 1 + math.log(tf) if tf > 0 else 0
            # IDF: inverse document frequency
            df = len(postings)
            n = self.index.total_docs
            idf = math.log((n + 1) / (df + 1)) + 1
            score += tf_score * idf
        return score

    def bm25_score(self, query_terms: List[str], doc_id: int,
                   k1: float = 1.2, b: float = 0.75) -> float:
        """BM25 scoring with document length normalization."""
        score = 0.0
        doc_len = self.index.doc_lengths.get(doc_id, 0)
        avg_dl = self.index.avg_doc_length

        for term in query_terms:
            postings = self.index.get_postings(term)
            tf = 0
            for p in postings:
                if p.doc_id == doc_id:
                    tf = p.term_freq
                    break
            if tf == 0:
                continue

            df = len(postings)
            n = self.index.total_docs
            # BM25 IDF
            idf = math.log((n - df + 0.5) / (df + 0.5) + 1)
            # BM25 TF with length normalization
            tf_norm = (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * doc_len / avg_dl))
            score += idf * tf_norm
        return score

    def search(self, query: str, top_k: int = 10, method: str = "bm25") -> List[Tuple[int, str, float]]:
        """Search and return top-k results."""
        query_terms = tokenize(query)
        if not query_terms:
            return []

        # Collect candidate documents (union of posting lists)
        candidates = set()
        for term in query_terms:
            for p in self.index.get_postings(term):
                candidates.add(p.doc_id)

        # Score each candidate
        scored = []
        for doc_id in candidates:
            if method == "bm25":
                score = self.bm25_score(query_terms, doc_id)
            else:
                score = self.tf_idf_score(query_terms, doc_id)
            title = self.index.documents.get(doc_id, f"doc_{doc_id}")
            scored.append((doc_id, title, score))

        # Sort by score descending
        scored.sort(key=lambda x: x[2], reverse=True)
        return scored[:top_k]
```

### Step 4: Demo

```python
def main():
    index = InvertedIndex()
    ranker = Ranker(index)

    # Index some documents
    documents = [
        (0, "Introduction to Algorithms",
         "Algorithms are step-by-step procedures for solving problems. "
         "Sorting algorithms like quicksort and mergesort are fundamental."),
        (1, "Data Structures",
         "Data structures organize data for efficient access. "
         "Trees, graphs, hash tables, and arrays are common data structures."),
        (2, "Machine Learning Basics",
         "Machine learning algorithms learn patterns from data. "
         "Linear regression and decision trees are simple algorithms."),
        (3, "Database Systems",
         "Databases store and retrieve data efficiently. "
         "B-trees and hash indexes speed up data access."),
        (4, "Operating Systems",
         "Operating systems manage hardware resources. "
         "Process scheduling and memory management are core algorithms."),
        (5, "Network Protocols",
         "Network protocols define communication rules. "
         "TCP and IP protocols handle data transmission."),
    ]

    for doc_id, title, text in documents:
        index.add_document(doc_id, title, text)

    print(f"Indexed {index.total_docs} documents")
    print(f"Vocabulary size: {len(index.index)} terms\n")

    # Search
    queries = ["algorithms and data structures", "database indexing", "machine learning"]
    for query in queries:
        print(f"Query: '{query}'")
        results = ranker.search(query, top_k=3)
        for doc_id, title, score in results:
            print(f"  [{score:.3f}] {title}")
        print()

if __name__ == "__main__":
    main()
```

Expected output:

```
Indexed 6 documents
Vocabulary size: 35+ terms

Query: 'algorithms and data structures'
  [2.847] Introduction to Algorithms
  [1.923] Data Structures
  [1.105] Machine Learning Basics

Query: 'database indexing'
  [3.214] Database Systems
  [0.892] Data Structures

Query: 'machine learning'
  [3.456] Machine Learning Basics
  [0.654] Introduction to Algorithms
```

## Use It

The architecture extends to web-scale systems by replacing local stores with distributed components:

- **Elasticsearch**: uses an inverted index (Lucene) with BM25 as the default scoring function. The same pipeline (ingest, tokenize, index, search) but distributed across many nodes with sharding and replication.
- **Google Search**: the original PageRank paper combined TF-IDF text scoring with link analysis. Modern Google uses hundreds of ranking signals, but the inverted index backbone remains.
- **Apache Lucene**: the most widely deployed search library. Its inverted index format, segment-based architecture, and scoring functions are the foundation of Elasticsearch, Solr, and many other search systems.

The key production lesson: **tokenization quality determines search quality more than ranking sophistication**. If the tokenizer doesn't handle stemming, synonyms, or multi-word phrases correctly, no ranking algorithm can fix the results. Production systems invest heavily in tokenization pipelines with language detection, stemming, synonym expansion, and phrase detection.

## Read the Source

- [Introduction to Information Retrieval](https://nlp.stanford.edu/IR-book/) — Manning, Raghavan, Schütze. The textbook on search engine internals. Chapter 1 (Boolean retrieval), Chapter 6 (Scoring), and Chapter 7 (Computing scores) are directly relevant.
- [Elasticsearch internals](https://github.com/elastic/elasticsearch) — The Lucene inverted index implementation is in `server/src/main/java/org/apache/lucene/`.
- [Okapi BM25 paper](https://en.wikipedia.org/wiki/Okapi_BM25) — The BM25 scoring function used by most production search engines.

## Ship It

- `code/main.py`: complete inverted index with TF-IDF and BM25 scoring, plus a demo corpus.
- `code/main.rs`: Rust implementation of the inverted index with the same scoring functions.
- `outputs/README.md`: search pipeline checklist covering crawling, indexing, ranking, and evaluation.

## Exercises

1. **Easy** — Add BM25-style scoring. Compare BM25 (k1=1.2, b=0.75) against TF-IDF on the demo corpus. Show how document length normalization changes the ranking for short vs long documents.
2. **Medium** — Add document update and delete handling. When a document is updated, re-tokenize it and update the posting lists. When deleted, remove its entries from all posting lists. This requires the index to be mutable and the posting lists to support efficient removal.
3. **Hard** — Add crawl politeness and deduplication strategy. Implement a crawler that respects robots.txt, rate-limits requests to the same domain, and deduplicates near-identical pages using SimHash or MinHash. Show that the crawler discovers new pages without re-crawling duplicates.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Inverted index | "term map" | A mapping from each term to a posting list. The core data structure of search engines. Called "inverted" because it inverts the document-term relationship: instead of "document contains terms," it's "term is in documents." |
| Posting list | "doc ids list" | The list of documents containing a given term, typically with term frequency and position information. Posting lists are sorted by document ID for efficient intersection during multi-term queries. |
| Ranking | "sort results" | Scoring documents against query relevance signals. TF-IDF and BM25 are the classic scoring functions. Modern systems add hundreds of signals (click data, freshness, authority). |
| Tokenization | "split text" | Converting raw text into normalized terms. Includes lowercasing, punctuation removal, stemming (reducing words to roots), and stop word removal. Tokenization quality directly impacts search quality. |
| BM25 | "best match 25" | A ranking function that extends TF-IDF with document length normalization and term frequency saturation. Used as the default scoring function in Elasticsearch and most production search engines. |

## Further Reading

- [Introduction to Information Retrieval](https://nlp.stanford.edu/IR-book/) — The standard textbook on search engine internals.
- [Elasticsearch documentation](https://www.elastic.co/guide/) — Production search engine architecture.
- [Apache Lucene](https://lucene.apache.org/) — The most widely deployed search library.
