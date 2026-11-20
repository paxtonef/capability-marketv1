# Contrat `cm.procurement.requirements.v1` — version 1.1.0

| Fichier | Rôle |
|---|---|
| `contracts/procurement_requirements_v1.json` | Contrat CPL B6, version 1.0.0 (inchangé) |
| `contracts/procurement_requirements_v1_1.json` | Contrat CPL B6, version 1.1.0 |
| `contracts/procurement_requirements_v1_1.schema.json` | JSON Schema (2020-12) décrivant le détail des éléments |

## Principe de compatibilité

La version 1.1.0 **n'ajoute aucun champ racine** et ne change aucun type racine.
L'extension tient entièrement dans les éléments des tableaux `requirements`,
`evidence` et `uncertainties`, que le contrat B6 type seulement comme `array`.

Conséquences :

- une charge utile 1.1.0 satisfait le contrat 1.0.0 tel qu'il est déclaré, et
  peut être enregistrée sous `schema_version="1.0.0"` sans installer quoi que ce
  soit dans CPL ;
- tout champ d'élément présent en 1.0.0 (`id`, `category`, `text`,
  `requirement_id`, `source_line`, `excerpt`, `text`) garde son nom, son type et
  sa valeur ;
- le fichier de contrat 1.1.0 a les mêmes `required_fields` et `field_types` que
  le 1.0.0 : seul `schema_version` diffère. Le détail est porté par le JSON Schema.

Le précédent V0.1 → V0.2 (ajout de `category` dans les éléments, sous la même
version 1.0.0) suit la même logique.

**Point d'attention pour un consommateur 1.0.0.** `requirements` contient
désormais aussi des énoncés non contraignants (`EXEMPTION`, `OPTIONAL`). Un
consommateur qui ne lit pas `category` doit filtrer sur `binding == true` pour
retrouver les catégories de V0.2.

## Structure

```json
{
  "tender_id": "CM-COMPLEX-001",
  "title": "Services linguistiques multilingues",
  "requirements": [
    {
      "id": "REQ-0005",
      "category": "CONDITIONAL_OBLIGATION",
      "text": "Si le volume dépasse 100 000 mots, le prestataire doit proposer un calendrier révisé.",
      "binding": true,
      "modality": "doit",
      "conditions": [
        { "type": "IF", "text": "Si le volume dépasse 100 000 mots",
          "source_line": 9, "byte_start": 329, "byte_end": 363 }
      ],
      "evidence_ids": ["EV-0005"],
      "uncertainty_ids": []
    }
  ],
  "evidence": [
    {
      "id": "EV-0005",
      "requirement_id": "REQ-0005",
      "source_line": 9,
      "excerpt": "Si le volume dépasse 100 000 mots, le prestataire doit proposer un calendrier révisé.",
      "column_start": 1,
      "byte_start": 329,
      "byte_end": 417
    }
  ],
  "uncertainties": [
    {
      "id": "UNC-0001",
      "kind": "PENDING_INFORMATION",
      "source_line": 12,
      "text": "La date limite de dépôt reste à confirmer.",
      "marker": "à confirmer",
      "byte_start": 534,
      "byte_end": 579,
      "requirement_id": null
    }
  ]
}
```

### `requirements[]`

| Champ | Type | 1.0.0 | Description |
|---|---|---|---|
| `id` | chaîne | oui | `REQ-0001`, séquentiel dans l'ordre du document |
| `category` | énumération | oui | `OBLIGATION`, `PROHIBITION`, `CONDITIONAL_OBLIGATION` ; ajoutés en 1.1.0 : `EXEMPTION`, `OPTIONAL` |
| `text` | chaîne | oui | Texte exact de l'énoncé source |
| `binding` | booléen | — | Vrai pour les trois catégories contraignantes |
| `modality` | chaîne | — | Marqueur de modalité, tel qu'écrit dans la source |
| `conditions` | tableau | — | Clauses conditionnelles ; vide s'il n'y en a pas |
| `evidence_ids` | tableau | — | Identifiants des preuves (une par exigence en V0.3) |
| `uncertainty_ids` | tableau | — | Identifiants des incertitudes portant sur l'exigence |

### `requirements[].conditions[]`

| Champ | Type | Description |
|---|---|---|
| `type` | énumération | `IF`, `WHEN`, `PROVIDED_THAT`, `EXCEPTION` |
| `text` | chaîne | Clause exacte, marqueur inclus |
| `source_line`, `byte_start`, `byte_end` | entiers | Position de la clause dans le fichier source |

### `evidence[]`

| Champ | Type | 1.0.0 | Description |
|---|---|---|---|
| `id` | chaîne | — | `EV-0001` |
| `requirement_id` | chaîne | oui | Exigence prouvée |
| `source_line` | entier | oui | Ligne, base 1 |
| `excerpt` | chaîne | oui | Extrait exact |
| `column_start` | entier | — | Colonne du premier caractère, base 1, en caractères |
| `byte_start`, `byte_end` | entiers | — | Décalages en octets dans le fichier source |

Invariant : `excerpt` est le décodage UTF-8 des octets `byte_start..byte_end` du
fichier source. Les décalages comptent le BOM éventuel ; la colonne ne le compte pas.

### `uncertainties[]`

| Champ | Type | 1.0.0 | Description |
|---|---|---|---|
| `id` | chaîne | — | `UNC-0001` |
| `kind` | énumération | — | `PENDING_INFORMATION`, `WEAK_MODALITY`, `VAGUE_TERM`, `AMBIGUOUS_NEGATION`, `MULTIPLE_MODALITIES`, `NO_REQUIREMENT_DETECTED` |
| `source_line` | entier ou nul | oui | Ligne, base 1 |
| `text` | chaîne | oui | Texte exact de l'énoncé |
| `marker` | chaîne ou nul | — | Marqueur déclencheur, tel qu'écrit dans la source |
| `byte_start`, `byte_end` | entiers ou nuls | — | Position de l'énoncé |
| `requirement_id` | chaîne ou nul | — | Exigence concernée, le cas échéant |

Seul `NO_REQUIREMENT_DETECTED` a une position nulle : c'est un avis du runner,
pas un extrait. En 1.0.0, ce même avis était une simple chaîne dans le tableau ;
en 1.1.0, tous les éléments sont des objets.

Les exemptions sont enregistrées dans `requirements` avec la catégorie
`EXEMPTION` et `binding: false`, avec leur preuve, comme toute autre exigence.

## Différences entre profils

| | 1.0.0 (`--contract-version 1.0.0`) | 1.1.0 (défaut) |
|---|---|---|
| Règles | V0.2, une exigence par ligne | V0.3, une exigence par phrase |
| Exemptions, possibilités | écartées | enregistrées (`binding: false`) |
| Conditions | préfixe de ligne uniquement | clause extraite et localisée |
| Localisation | ligne | ligne, colonne, octets |
| Incertitudes | ligne et texte | typées, localisées, reliées aux exigences |

## Évolutions futures

Tout ajout de champ dans un élément reste compatible et relève d'une version
mineure. L'ajout d'un champ racine devrait être déclaré dans `field_types` et
vérifié contre CPL B6 avant adoption.
