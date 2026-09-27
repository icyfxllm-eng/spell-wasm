# Bake-off — pt

60 words per voice, both recognizers, blind (I3).

| Voice | Pass | Weak | Fail |
|---|---|---|---|
| pt-BR-Neural2-B **(current)** | 45 | 9 | 6 |
| pt-BR-Wavenet-B | 44 | 9 | 7 |
| pt-BR-Wavenet-E | 41 | 13 | 6 |
| pt-BR-Neural2-A | 40 | 12 | 8 |
| pt-BR-Wavenet-D | 34 | 19 | 7 |
| pt-BR-Wavenet-A | 32 | 19 | 9 |
| pt-BR-Wavenet-C | 32 | 19 | 9 |
| pt-BR-Neural2-C | 31 | 20 | 9 |

## Missed by both recognizers

### pt-BR-Neural2-B

- sete -> "7." / "7"
- dylan -> "Dila" / "D"
- expulso -> "Espulso" / "expul"
- high -> "Hi!" / ""
- portáteis -> "Portades" / "portades"
- wikipedia -> "o Iquipédia" / "Wikipédia"

### pt-BR-Wavenet-B

- sete -> "7." / "7"
- outono -> "O tono." / "outo"
- dylan -> "Dila" / "D"
- expulso -> "Espulso" / "expul"
- high -> "Hi!" / ""
- portáteis -> "Portades" / "portades"
- wikipedia -> "O Iquipédia." / "Wikipédia"

### pt-BR-Wavenet-E

- sete -> "7." / "7"
- dylan -> "Gila!" / "dyla"
- high -> "Oi!" / "hi"
- wikipedia -> "O Iquipédia." / "Wikipédia"
- leoa -> "Leor" / "Léo"
- soltos -> "Soldos" / "soldos"

### pt-BR-Neural2-A

- sete -> "Sachi" / ""
- meia -> "Mia!" / "me"
- dylan -> "De lu" / "dy"
- high -> "Hãi!" / ""
- bebem -> "MAMBIN" / "bebe"
- wikipedia -> "o Iquipédia" / "Wikipédia"
- leoa -> "Leu!" / "Léo"
- tecla -> "Técua!" / "Tec"

### pt-BR-Wavenet-D

- sete -> "7." / "7"
- vendas -> "Vem-das!" / "venda"
- fêmeas -> "Famous." / "fêmea"
- dylan -> "Dila." / "dyla"
- over -> "Louve!" / "vou"
- high -> "Oi!" / ""
- wikipedia -> "o Iquipédia." / "Wikipédia"

### pt-BR-Wavenet-A

- sete -> "7." / ""
- outono -> "O tono." / "outo"
- esportes -> "Espartes" / "Esporte"
- dylan -> "Dila" / "dia"
- high -> "Oi!" / ""
- portáteis -> "Portátis" / "porta-te"
- wikipedia -> "O Iquipédia." / "Wikipédia"
- leoa -> "Leo!" / "Léo"
- tecla -> "Tadpla!" / "Tec"

### pt-BR-Wavenet-C

- sete -> "7." / ""
- outono -> "O tono." / "outo"
- esportes -> "Espartes" / "Esporte"
- dylan -> "Dila" / "dia"
- high -> "Oi!" / ""
- portáteis -> "Portátis" / "porta-te"
- wikipedia -> "O Iquipédia." / "Wikipédia"
- leoa -> "Leo!" / "Léo"
- tecla -> "Tadpla!" / "Tec"

### pt-BR-Neural2-C

- sete -> "7." / ""
- outono -> "O tono." / "outo"
- esportes -> "Espartes" / "Esporte"
- dylan -> "Dila" / "dia"
- high -> "Oi!" / ""
- portáteis -> "Portátis" / "porta-te"
- wikipedia -> "O Iquipédia." / "Wikipédia"
- leoa -> "Leo!" / "Léo"
- tecla -> "Tadpla!" / "Tec"


## Split decisions — one recognizer heard it, one did not

The `half` case lives here, not above: a word only one machine gets is the
one a listener is most likely to find ambiguous, and a report that showed
only total failures left it out entirely.

### pt-BR-Neural2-B

- meia -> "Mia!" / "meia"
- carregar -> "Cahegar" / "carregar"
- levar -> "Lêvar" / "levar"
- garrafa -> "Gahaf" / "garrafa"
- advogados -> "Adivogados" / "Advogados"
- outono -> "Outono!" / "outo"
- over -> "Over." / "ou"
- bebem -> "Bebem!" / "bebe"
- tecla -> "Teclado." / "tecla"

### pt-BR-Wavenet-B

- pensar -> "PENSAR" / ""
- meia -> "Mia!" / "meia"
- carregar -> "Cahegar" / "carregar"
- levar -> "Lêvar" / "levar"
- garrafa -> "Gahaf" / "garrafa"
- advogados -> "Adivogados!" / "Advogados"
- over -> "Over." / "ou"
- bebem -> "Bebem!" / "bebe"
- tecla -> "Teclado." / "tecla"

### pt-BR-Wavenet-E

- luz -> "Luz." / ""
- porco -> "Porco!" / "por"
- pensar -> "Pensa!" / "pensar"
- trazer -> "Trazir!" / "trazer"
- igreja -> "Ingreja." / "igreja"
- carregar -> "Cá é gar." / "carregar"
- garrafa -> "Gahafa" / "garrafa"
- seguiu -> "Segue-o!" / "seguiu"
- advogados -> "Adivogados" / "Advogados"
- over -> "Over!" / ""
- portáteis -> "Portátis" / "portáteis"
- permanecer -> "Permanisser" / "permanecer"
- tecla -> "Décla." / "tecla"

### pt-BR-Neural2-A

- luz -> "Luz" / ""
- pensar -> "Pensa-se!" / "pensar"
- favor -> "Por favor." / "favor"
- vendas -> "Vindas!" / "vendas"
- seguiu -> "Seguiu!" / "segu"
- visão -> "Visão!" / "Visa"
- viveu -> "Viveu!" / "Vivi"
- advogados -> "Adivogados" / "Advogados"
- outono -> "ou tono." / "outono"
- over -> "Over!" / "ou"
- portáteis -> "Portáteis" / "porta-te"
- jornalístico -> "Jornalista" / "jornalístico"

### pt-BR-Wavenet-D

- andar -> "Anda!" / "andar"
- luz -> "Luz!" / ""
- porco -> "Porco!" / "por"
- pensar -> "Pensa!" / "pensar"
- trazer -> "Trazê" / "trazer"
- folhas -> "Folhas" / "folha"
- carregar -> "Cajé-ga!" / "carregar"
- levar -> "Leva!" / "levar"
- felizes -> "Felizes!" / "feli"
- seguiu -> "Seguiu!" / "segu"
- advogados -> "Adivogados" / "Advogados"
- envolvidas -> "envolvidas" / "envolvida"
- outono -> "ou tono." / "outono"
- esportes -> "Esportes" / "Esporte"
- portáteis -> "Portáteis" / "portá"
- permanecer -> "Permanense!" / "permanecer"
- leoa -> "Leo!" / "leoa"
- tecla -> "Thank you!" / "tecla"
- soltos -> "soltos!" / "sol"

### pt-BR-Wavenet-A

- andar -> "Andar!" / ""
- azul -> "Azor" / "azul"
- luz -> "Luz." / ""
- porco -> "Porco" / "por"
- igreja -> "Ingreja!" / "igreja"
- meia -> "Meia!" / ""
- carregar -> "Cá é gár." / "carregar"
- favor -> "Por favor!" / "favor"
- vendas -> "Vem des." / "vendas"
- garrafa -> "Bia rafa." / "garrafa"
- seguiu -> "Seguiu!" / "segu"
- visão -> "Visão." / "Visa"
- viveu -> "Viveu!" / "Vivi"
- advogados -> "Adivogados" / "Advogados"
- expulso -> "Espulso" / "expulso"
- saudita -> "Saudita!" / "saud"
- over -> "Over." / "ou"
- bebem -> "Bebem!" / "bebe"
- soltos -> "Soltus" / "soltos"

### pt-BR-Wavenet-C

- andar -> "Andar!" / ""
- azul -> "Azor" / "azul"
- luz -> "Luz." / ""
- porco -> "Porco" / "por"
- igreja -> "Ingreja!" / "igreja"
- meia -> "Meia!" / ""
- carregar -> "Cá é gár." / "carregar"
- favor -> "Por favor!" / "favor"
- vendas -> "Vem des." / "vendas"
- garrafa -> "Bia rafa." / "garrafa"
- seguiu -> "Seguiu!" / "segu"
- visão -> "Visão." / "Visa"
- viveu -> "Viveu!" / "Vivi"
- advogados -> "Adivogados" / "Advogados"
- expulso -> "Espulso" / "expulso"
- saudita -> "Saudita!" / "saud"
- over -> "Over." / "ou"
- bebem -> "Bebem!" / "bebe"
- soltos -> "Soltus" / "soltos"

### pt-BR-Neural2-C

- andar -> "Andar!" / ""
- azul -> "Azor" / "azul"
- luz -> "Luz." / ""
- porco -> "Porco" / "por"
- igreja -> "Ingreja!" / "igreja"
- meia -> "Meia!" / ""
- carregar -> "Cá é gár." / "carregar"
- favor -> "Por favor!" / "favor"
- vendas -> "Vem des." / "vendas"
- garrafa -> "Bia rafa." / "garrafa"
- seguiu -> "Seguiu!" / "segu"
- visão -> "Visão." / "Visa"
- fêmeas -> "Fanias" / "fêmeas"
- viveu -> "Viveu!" / "Vivi"
- advogados -> "Adivogados" / "Advogados"
- expulso -> "Espulso" / "expulso"
- saudita -> "Saudita!" / "saud"
- over -> "Over." / "ou"
- bebem -> "Bebem!" / "bebe"
- soltos -> "Soltus" / "soltos"


**Recommendation only.** Switching a language's default voice needs
Eric's signature for that language (D6), and the switch itself is atomic:
every clip regenerates and passes F1 and F2 before any player hears one
(F4 step 5), one language at a time, never mid-session (D16).
