"""
Vector Databases — HNSW, IVF, PQ
Phase 10 — Databases & Storage Systems

IVF, PQ, and IVF+PQ from scratch with benchmark.

Requirements: numpy, scikit-learn
"""

import time
import numpy as np
from sklearn.cluster import KMeans


class IVF:
    """Inverted File Index with k-means clustering."""

    def __init__(self, nlist: int = 100):
        self.nlist = nlist
        self.centroids = None
        self.inverted_lists = None
        self.vectors = None

    def train(self, vectors: np.ndarray):
        self.vectors = vectors
        kmeans = KMeans(n_clusters=self.nlist, random_state=42, n_init="auto")
        kmeans.fit(vectors)
        self.centroids = kmeans.cluster_centers_
        self.inverted_lists = [[] for _ in range(self.nlist)]
        for i, label in enumerate(kmeans.labels_):
            self.inverted_lists[label].append(i)

    def search(
        self, query: np.ndarray, k: int, nprobe: int = 10
    ) -> list[tuple[int, float]]:
        dists = np.linalg.norm(self.centroids - query, axis=1)
        nearest = np.argsort(dists)[:nprobe]
        candidates: dict[int, float] = {}
        for cid in nearest:
            for vid in self.inverted_lists[cid]:
                if vid not in candidates:
                    candidates[vid] = float(np.linalg.norm(self.vectors[vid] - query))
        sorted_candidates = sorted(candidates.items(), key=lambda x: x[1])
        return sorted_candidates[:k]


class PQ:
    """Product Quantization with asymmetric distance computation."""

    def __init__(self, M: int = 8, Ks: int = 256):
        self.M = M
        self.Ks = Ks
        self.D = None
        self.sub_dim = None
        self.codebooks: list[np.ndarray] | None = None
        self.codes: np.ndarray | None = None

    def train(self, vectors: np.ndarray):
        N, D = vectors.shape
        self.D = D
        assert D % self.M == 0, f"D={D} must be divisible by M={self.M}"
        self.sub_dim = D // self.M
        self.codebooks = []
        code_list = []
        for m in range(self.M):
            sub = vectors[:, m * self.sub_dim : (m + 1) * self.sub_dim]
            kmeans = KMeans(n_clusters=self.Ks, random_state=42, n_init="auto")
            kmeans.fit(sub)
            self.codebooks.append(kmeans.cluster_centers_)
            code_list.append(kmeans.predict(sub))
        self.codes = np.column_stack(code_list).astype(np.uint8)

    def encode(self, vectors: np.ndarray) -> np.ndarray:
        N = vectors.shape[0]
        codes = np.zeros((N, self.M), dtype=np.uint8)
        for m in range(self.M):
            sub = vectors[:, m * self.sub_dim : (m + 1) * self.sub_dim]
            dists = np.linalg.norm(
                sub[:, np.newaxis, :] - self.codebooks[m][np.newaxis, :, :], axis=2
            )
            codes[:, m] = np.argmin(dists, axis=1)
        return codes

    def adc(self, query: np.ndarray, codes: np.ndarray) -> np.ndarray:
        dists = np.zeros(codes.shape[0])
        for m in range(self.M):
            q_sub = query[m * self.sub_dim : (m + 1) * self.sub_dim]
            lut = np.linalg.norm(self.codebooks[m] - q_sub, axis=1)
            dists += lut[codes[:, m].astype(int)]
        return dists


class IVF_PQ:
    """IVF for coarse search + PQ for approximate distances."""

    def __init__(self, nlist: int = 100, M: int = 8, Ks: int = 256):
        self.ivf = IVF(nlist=nlist)
        self.pq = PQ(M=M, Ks=Ks)
        self.codes = None

    def train(self, vectors: np.ndarray):
        self.ivf.train(vectors)
        self.pq.train(vectors)
        self.codes = self.pq.encode(vectors)

    def search(
        self, query: np.ndarray, k: int, nprobe: int = 10
    ) -> list[tuple[int, float]]:
        dists = np.linalg.norm(self.ivf.centroids - query, axis=1)
        nearest = np.argsort(dists)[:nprobe]
        cand_ids: list[int] = []
        for cid in nearest:
            cand_ids.extend(self.ivf.inverted_lists[cid])
        if not cand_ids:
            return []
        cand_codes = self.codes[cand_ids]
        pq_dists = self.pq.adc(query, cand_codes)
        idx = np.argsort(pq_dists)[:k]
        return [(cand_ids[i], pq_dists[i]) for i in idx]


def exact_search(
    database: np.ndarray, query: np.ndarray, k: int
) -> list[tuple[int, float]]:
    dists = np.linalg.norm(database - query, axis=1)
    idx = np.argsort(dists)[:k]
    return [(int(i), float(dists[i])) for i in idx]


def recall_at_k(
    exact: list[tuple[int, float]], approx: list[tuple[int, float]], k: int
) -> float:
    exact_set = set(i for i, _ in exact[:k])
    approx_set = set(i for i, _ in approx[:k])
    if not exact_set:
        return 1.0
    return len(exact_set & approx_set) / len(exact_set)


def generate_clustered_data(
    n: int, d: int, n_clusters: int, std: float = 0.4
) -> np.ndarray:
    """Generate synthetic data with clear cluster structure so ANN methods work."""
    rng = np.random.default_rng(42)
    centroids = rng.uniform(-5, 5, size=(n_clusters, d))
    per_cluster = n // n_clusters
    vectors = []
    for c in centroids:
        cluster_pts = c + rng.normal(0, std, size=(per_cluster, d))
        vectors.append(cluster_pts)
    remainder = n - per_cluster * n_clusters
    if remainder > 0:
        vectors.append(rng.uniform(-5, 5, size=(remainder, d)))
    return np.vstack(vectors).astype(np.float32)


def benchmark():
    np.random.seed(42)
    N_train, N_test, D, k = 10_000, 100, 128, 10
    n_clusters = 50

    print(f"Generating {N_train} train + {N_test} test vectors (clustered data, D={D})...")
    train_vectors = generate_clustered_data(N_train, D, n_clusters, std=0.4)
    test_vectors = generate_clustered_data(N_test, D, n_clusters // 2, std=0.4)

    # --- Exact search baseline ---
    t0 = time.perf_counter()
    exact_results = []
    for q in test_vectors:
        exact_results.append(exact_search(train_vectors, q, k))
    exact_time = time.perf_counter() - t0
    print(f"\nExact k-NN:   {exact_time:.3f}s")

    # --- IVF ---
    ivf = IVF(nlist=100)
    t0 = time.perf_counter()
    ivf.train(train_vectors)
    train_time = time.perf_counter() - t0

    t0 = time.perf_counter()
    ivf_results = []
    for q in test_vectors:
        ivf_results.append(ivf.search(q, k, nprobe=10))
    ivf_time = time.perf_counter() - t0

    ivf_recall = np.mean(
        [recall_at_k(ex, ap, k) for ex, ap in zip(exact_results, ivf_results)]
    )
    print(f"IVF (nlist=100, nprobe=10):  {ivf_time:.3f}s  recall@{k}={ivf_recall:.3f}  (train: {train_time:.3f}s)")

    # --- IVF + PQ ---
    ivfpq = IVF_PQ(nlist=100, M=8)
    t0 = time.perf_counter()
    ivfpq.train(train_vectors)
    train_time_pq = time.perf_counter() - t0

    t0 = time.perf_counter()
    ivfpq_results = []
    for q in test_vectors:
        ivfpq_results.append(ivfpq.search(q, k, nprobe=10))
    ivfpq_time = time.perf_counter() - t0

    ivfpq_recall = np.mean(
        [recall_at_k(ex, ap, k) for ex, ap in zip(exact_results, ivfpq_results)]
    )
    print(
        f"IVF+PQ (nlist=100, nprobe=10, M=8):  {ivfpq_time:.3f}s  recall@{k}={ivfpq_recall:.3f}  "
        f"(train: {train_time_pq:.3f}s)"
    )

    # --- Larger benchmark: 100K vectors ---
    print(f"\n--- Scaling to 100K vectors ---")
    N_big = 100_000
    big_vectors = generate_clustered_data(N_big, D, 100, std=0.4)
    big_queries = generate_clustered_data(20, D, 10, std=0.4)

    ivf_big = IVF(nlist=200)
    t0 = time.perf_counter()
    ivf_big.train(big_vectors)
    print(f"IVF train (100K):  {time.perf_counter() - t0:.3f}s")

    t0 = time.perf_counter()
    big_exact = []
    for q in big_queries:
        big_exact.append(exact_search(big_vectors, q, k))
    exact_big_time = time.perf_counter() - t0

    t0 = time.perf_counter()
    big_ivf_results = []
    for q in big_queries:
        big_ivf_results.append(ivf_big.search(q, k, nprobe=20))
    ivf_big_time = time.perf_counter() - t0

    big_recall = np.mean(
        [recall_at_k(ex, ap, k) for ex, ap in zip(big_exact, big_ivf_results)]
    )
    print(f"Exact k-NN (100K):  {exact_big_time:.3f}s")
    print(
        f"IVF (nlist=200, nprobe=20, 100K):  {ivf_big_time:.3f}s  recall@{k}={big_recall:.3f}"
    )

    print(f"\nSpeedup vs exact:  {exact_time / ivf_time:.1f}x (10K IVF), "
          f"{exact_big_time / ivf_big_time:.1f}x (100K IVF)")


if __name__ == "__main__":
    benchmark()
