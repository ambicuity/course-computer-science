# PBR Reference Card

Quick-lookup reference for physically based rendering equations and material parameters.

## Core Equations

### Rendering Equation

```
Lo(p,ωo) = Le(p,ωo) + ∫_Ω fr(p,ωi,ωo) Li(p,ωi) (n·ωi) dωi
```

### Cook-Torrance Specular BRDF

```
         D(h) · G(ωi,ωo) · F(ωo,h)
fr = ─────────────────────────────────
           4 · (n·ωi) · (n·ωo)
```

### GGX / Trowbridge-Reitz NDF

```
           α²
D(h) = ───────────────────────
        π · ((n·h)²(α²-1)+1)²

where α = roughness²
```

### Smith Geometry Function (Schlick-GGX)

```
G_Smith(ωi,ωo) = G1(ωi) · G1(ωo)

           n·v
G1_Schlick(v) = ───────────────
                (n·v)(1-k)+k

Direct lighting:  k = (roughness+1)² / 8
IBL:              k = roughness² / 2
```

### Schlick Fresnel Approximation

```
F(cosθ) = F0 + (1-F0)(1-cosθ)⁵
```

### Complete PBR BRDF (Metallic-Roughness)

```
fr = (1-metallic) · albedo/π + CookTorrance

F0 = mix(0.04, albedo, metallic)
kD = (1-F) · (1-metallic)      // diffuse contribution weight
kS = F                           // specular already in Cook-Torrance
```

## F0 Reference Values (Linear RGB)

### Dielectrics (metallic = 0)

| Material     | F0 (R, G, B)               | F0 approx |
|-------------|----------------------------|-----------|
| Water        | (0.02, 0.02, 0.02)         | 0.02      |
| Plastic      | (0.03–0.05, same, same)    | ~0.04     |
| Glass        | (0.04, 0.04, 0.04)         | 0.04      |
| Diamond      | (0.15, 0.15, 0.15)         | 0.15      |
| Skin         | (0.028, 0.028, 0.028)      | 0.028     |
| Leaves       | (0.028, 0.028, 0.028)      | 0.028     |

### Metals (metallic = 1, F0 = albedo)

| Material     | F0 / Albedo (R, G, B)      | Note             |
|-------------|----------------------------|------------------|
| Gold         | (1.00, 0.76, 0.34)         | Warm yellow tint |
| Copper       | (0.95, 0.63, 0.54)         | Warm red tint    |
| Aluminum     | (0.91, 0.92, 0.92)         | Near achromatic  |
| Iron         | (0.56, 0.57, 0.58)         | Slightly warm    |
| Chrome       | (0.55, 0.55, 0.55)         | Achromatic       |
| Silver       | (0.97, 0.97, 0.97)         | Near white       |
| Titanium     | (0.54, 0.50, 0.45)         | Slightly warm    |
| Platinum     | (0.67, 0.64, 0.60)         | Slightly warm    |

## Material Presets

| Preset       | Albedo (RGB)               | Metallic | Roughness |
|-------------|----------------------------|----------|-----------|
| Red Plastic | (0.8, 0.1, 0.1)           | 0.0      | 0.4       |
| Gold        | (1.0, 0.76, 0.34)         | 1.0      | 0.3       |
| Chrome      | (0.55, 0.55, 0.55)        | 1.0      | 0.15      |
| Rubber      | (0.2, 0.2, 0.2)           | 0.0      | 0.9       |
| Copper      | (0.95, 0.63, 0.54)        | 1.0      | 0.25      |
| White Tile  | (0.9, 0.9, 0.9)           | 0.0      | 0.7       |
| Aluminum    | (0.91, 0.92, 0.92)        | 1.0      | 0.2       |
| Leather     | (0.3, 0.15, 0.05)         | 0.0      | 0.6       |

## Key Constraints

- **Energy Conservation:** ∫ fr · cosθ dω ≤ 1
- **Helmholtz Reciprocity:** fr(ωi,ωo) = fr(ωo,ωi)
- **Non-negativity:** fr ≥ 0
- **Dielectric rule:** F0 ≈ 0.04 (achromatic)
- **Metal rule:** No diffuse, F0 = albedo (chromatic)
- **Roughness range:** 0 (mirror) to 1 (maximum diffuse spread)
- **Roughness is squared for NDF:** α = roughness²

## Quick Derivation Checklist

When evaluating Cook-Torrance for a fragment:

1. Compute H = normalize(V + L)
2. NdotH, NdotV, NdotL, VdotH = clamp(dot products, 0, 1)
3. F0 = mix(0.04, albedo, metallic)
4. D = GGX(NdotH, roughness)
5. G = Smith(NdotV, NdotL, roughness)
6. F = Schlick(VdotH, F0)
7. specular = D·G·F / (4·NdotV·NdotL)
8. kD = (1-F) · (1-metallic)
9. diffuse = kD · albedo / π
10. Lo += (diffuse + specular) · radiance · NdotL