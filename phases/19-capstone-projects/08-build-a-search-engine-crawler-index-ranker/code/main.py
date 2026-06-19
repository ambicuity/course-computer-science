# Build a Search Engine (Crawler + Index + Ranker)
# Run: python3 main.py
#
# Architecture:
#   Documents → Tokenizer → Inverted Index → Ranker (TF-IDF / BM25) → Results
#
# Implements a complete search engine with tokenization, positional inverted index,
# TF-IDF scoring, and BM25 ranking.

import re
import math
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Tuple

# =============================================================================
# Step 1: Tokenizer
# =============================================================================

def tokenize(text: str) -> List[str]:
    """Normalize text into tokens: lowercase, split on non-alphanumeric."""
    text = text.lower()
    tokens = re.findall(r'[a-z0-9]+', text)
    stop_words = {'the', 'a', 'an', 'is', 'are', 'was', 'were', 'in', 'on',
                  'at', 'to', 'for', 'of', 'and', 'or', 'but', 'not', 'with',
                  'this', 'that', 'it', 'by', 'from', 'as', 'be', 'has', 'had'}
    return [t for t in tokens if t not in stop_words and len(t) > 1]

# =============================================================================
# Step 2: Inverted Index
# =============================================================================

@dataclass
class Posting:
    doc_id: int
    term_freq: int
    positions: List[int] = field(default_factory=list)

class InvertedIndex:
    def __init__(self):
        self.index: Dict[str, List[Posting]] = defaultdict(list)
        self.documents: Dict[int, str] = {}
        self.doc_lengths: Dict[int, int] = {}
        self.total_docs: int = 0
        self.avg_doc_length: float = 0.0

    def add_document(self, doc_id: int, title: str, text: str):
        tokens = tokenize(text)
        self.documents[doc_id] = title
        self.doc_lengths[doc_id] = len(tokens)
        self.total_docs += 1

        term_positions: Dict[str, List[int]] = defaultdict(list)
        for pos, token in enumerate(tokens):
            term_positions[token].append(pos)

        for term, positions in term_positions.items():
            self.index[term].append(Posting(
                doc_id=doc_id, term_freq=len(positions), positions=positions
            ))

        total_len = sum(self.doc_lengths.values())
        self.avg_doc_length = total_len / self.total_docs

    def get_postings(self, term: str) -> List[Posting]:
        return self.index.get(term.lower(), [])

    def document_frequency(self, term: str) -> int:
        return len(self.get_postings(term))

# =============================================================================
# Step 3: Ranker (TF-IDF and BM25)
# =============================================================================

class Ranker:
    def __init__(self, index: InvertedIndex):
        self.index = index

    def tf_idf_score(self, query_terms: List[str], doc_id: int) -> float:
        score = 0.0
        for term in query_terms:
            postings = self.index.get_postings(term)
            tf = 0
            for p in postings:
                if p.doc_id == doc_id:
                    tf = p.term_freq
                    break
            if tf == 0: continue
            tf_score = 1 + math.log(tf) if tf > 0 else 0
            df = len(postings)
            n = self.index.total_docs
            idf = math.log((n + 1) / (df + 1)) + 1
            score += tf_score * idf
        return score

    def bm25_score(self, query_terms: List[str], doc_id: int,
                   k1: float = 1.2, b: float = 0.75) -> float:
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
            if tf == 0: continue
            df = len(postings)
            n = self.index.total_docs
            idf = math.log((n - df + 0.5) / (df + 0.5) + 1)
            tf_norm = (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * doc_len / avg_dl))
            score += idf * tf_norm
        return score

    def search(self, query: str, top_k: int = 10, method: str = "bm25") -> List[Tuple[int, str, float]]:
        query_terms = tokenize(query)
        if not query_terms: return []
        candidates = set()
        for term in query_terms:
            for p in self.index.get_postings(term):
                candidates.add(p.doc_id)
        scored = []
        for doc_id in candidates:
            score = self.bm25_score(query_terms, doc_id) if method == "bm25" else self.tf_idf_score(query_terms, doc_id)
            title = self.index.documents.get(doc_id, f"doc_{doc_id}")
            scored.append((doc_id, title, score))
        scored.sort(key=lambda x: x[2], reverse=True)
        return scored[:top_k]

# =============================================================================
# Step 4: Demo
# =============================================================================

def main():
    index = InvertedIndex()
    ranker = Ranker(index)

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

    queries = ["algorithms and data structures", "database indexing", "machine learning"]
    for query in queries:
        print(f"Query: '{query}'")
        results = ranker.search(query, top_k=3)
        for doc_id, title, score in results:
            print(f"  [{score:.3f}] {title}")
        print()

if __name__ == "__main__":
    main()
