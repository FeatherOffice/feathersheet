# FeatherSheet — arquitetura Rust com Iced

**Direção atual — 07/10/2026: motor e interface 100% Rust, com Iced.**
Prioridade: **FeatherSlide → FeatherDocs → FeatherSheet**; FeatherCAD
depois, se houver tempo. O inventário abaixo registra funcionalidades e
decisões anteriores; a migração da interface deste aplicativo ainda deve
ser conferida no código, sem presumir que os recursos já existem em Iced.

## Por que

O motivo é medido, não estético. Os três gargalos que o `ROADMAP.md`
descreve — o rodapé somando a seleção, a cópia de segurança serializando
tudo, o `.fshz` passando por `toJSON()` — são todos a mesma coisa: **a thread
que desenha é a mesma que calcula**. Cada conserto foi um orçamento de tempo,
um trabalho fatiado em quadros de 8 ms, uma política para desligar sozinho
acima de 300 mil células. São remendos honestos contra um limite que não
cede.

Do outro lado, o que já foi para o Rust ganhou de goleada: o `.xlsx` e o
`.fshz` nativos abriram arquivo grande sem travar a janela, e a formatação de
data na mão derrubou 1 milhão de células de 65 s para 2,9 s. O padrão se
repete todas as vezes.

Com o motor fora da `webview` isso deixa de ser exceção: cálculo pesado não
disputa espaço com o desenho, a memória para de ser um `Map` de objetos do
JavaScript, e o paralelismo de verdade (várias threads recalculando colunas
independentes) passa a ser possível — nada disso existe numa aba de
navegador.

## Quem é dono do quê

A regra é curta: **quem guarda, calcula; quem desenha, esquece.**

| Lado | É dono de |
| --- | --- |
| **Rust** | pasta de trabalho, abas, células, valores, estilos, larguras e alturas; tokenizar, compilar e avaliar fórmula; grafo de dependências, recálculo, derramamento; desfazer e refazer; ordenar, filtrar, localizar e substituir, duplicadas, dividir texto, inserir e excluir; formatação condicional, validação, intervalos nomeados; consulta (OpenQuery) e tabela dinâmica; ler e gravar `.fshz`, `.xlsx`, CSV e JSON; macros; **e o texto que aparece na célula** |

Duas consequências que não são óbvias:

## A ponte

Três tipos de tráfego, e cada um tem um formato porque o custo de cada um é
diferente.

### 1. Comandos — pergunta e resposta, em JSON

```
celula_gravar(aba, linha, coluna, texto)    -> { versao, retangulos_sujos }
aba_inserir(nome)                           -> { id, versao }
ordenar(aba, retangulo, colunas)            -> { versao, retangulos_sujos }
estatisticas_coluna(aba, coluna)            -> { soma, media, minimo, maximo, contagem }
desfazer() / refazer()                      -> { versao, retangulos_sujos }
```

### 2. O quadro — o retângulo visível, em binário

É o caminho quente: acontece a cada rolagem, a cada tecla, a cada redesenho.
JSON aqui seria um erro — milhares de objetos por quadro, criados e jogados
fora sessenta vezes por segundo.

```
cabeçalho   "BLC1", versão do conteúdo, r0, c0, r1, c1, quantidade
colunas     linha[], coluna[], bandeiras[], estilo[], início do texto[], tamanho[]
textos      um bloco UTF-8 com todos os textos já formatados, emendados
estilos     dicionário dos estilos citados neste quadro
```

Só célula com conteúdo entra. As bandeiras dizem o que o desenho precisa
saber sem olhar o texto: se é erro, se é número (para alinhar à direita), se
é fórmula, se é célula derramada, se tem formatação condicional por cima. Se
a `versao_que_tenho` ainda vale, a resposta é um cabeçalho de oito bytes
dizendo "nada mudou" — o caso comum quando a pessoa só mexeu o mouse.

O que não cabe em pergunta e resposta, porque acontece quando o Rust decide:

| Evento | Quando |
| --- | --- |
| `planilha_mudou` | recálculo terminou; vem a versão e os retângulos sujos |
| `andamento` | trabalho longo: quanto já foi, para a barra e o "processando" |

## A ordem da migração

| Etapa | O que vai | Situação |
| --- | --- | --- |
| 0 | `refs`, `values`, `number`, `tokenizer`, `ast`, `parser` | ✅ 21/09/2026, 55 testes |
| 1a | `evaluator`, `fn_helpers`, `functions` — o contrato | ✅ 21/09/2026, 103 testes |
| 2 | armazenamento: `sheet`, `data-table`, `column-store`, `formula-table`, `sheet-deps`, `sheet-spill`, `workbook` | ✅ 22/09/2026 |
| 6 | apagar o `src/core/`, o `vite` de arquivo único e a versão web | a fazer |

## Regras que continuam valendo

As do `CLAUDE.md` não mudam de lado: responder em português, um componente por
arquivo, nada de item de menu morto, todo bug reportado vira teste, mudanças
pequenas. Acrescentam-se duas, do lado Rust:
