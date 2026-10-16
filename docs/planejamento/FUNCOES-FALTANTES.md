# Funções do Excel que faltam no motor

> Levantado em 13/10/2026. Comparação nome por nome entre a tabela `FUNCOES`
> de `crates/feathersheet-core/src/funcoes/functions.rs` (mais `LET`,
> `LAMBDA`, `MAP` e `REDUCE`, que o avaliador resolve pelo nome) e a lista
> alfabética de funções do Excel 365, montada à mão por categoria (520 nomes).
> Apelidos em português não contam: só o nome canônico em inglês.

**Temos 457 das 520 funções do Excel; faltam 63.** (Atualizado em 15/10/2026: entraram
48, ver o fim deste arquivo. Em 14/10: PERCENTOF, CHITEST, TTEST e os 26 números complexos IM*/COMPLEX.) Fora da lista do Excel,
o motor tem ainda `COUNTUNIQUE`, `JOIN`, `REGEXMATCH`, `IMPORTDATA`,
`IMPORTJSON`, `FETCHTEXT` e `IMAGE.BASE64`.

Quando uma função daqui for portada, tirar a linha desta lista.

## Prioridade sugerida

1. **Referência e lambdas** (o que falta das mais usadas): `OFFSET`, `INDIRECT`
   (precisam de referência dinâmica no avaliador), `BYROW`, `BYCOL`, `SCAN`,
   `MAKEARRAY`, `ISFORMULA`, `FORMULATEXT`, `SHEET`, `SHEETS`, `GETPIVOTDATA`.
2. **Regressão**: `LINEST`, `LOGEST` (TENDÊNCIA e CRESCIMENTO já existem para uma variável).
3. **Financeiras de cupom** (`COUP*`, `PRICE`, `YIELD`, `DURATION`, `ODD*`) e
   **engenharia** (`BESSELK`, `BESSELY`).
4. **Por último ou nunca**: cubo/OLAP, suplementos (`CALL`, `REGISTER.ID`,
   `EUROCONVERT`), web (`WEBSERVICE`, `FILTERXML` — pela regra
   de privacidade só fariam sentido por pedido explícito), `STOCKHISTORY`,
   `TRANSLATE`/`DETECTLANGUAGE` (dependem de serviço on-line) e as de texto
   de byte duplo (`ASC`, `DBCS`, `PHONETIC`, `BAHTTEXT`).

## Lista completa, por categoria

### Pesquisa e referência (9)

`AREAS`, `FORMULATEXT`, `GETPIVOTDATA`, `GROUPBY`, `INDIRECT`, `OFFSET`, 
`PIVOTBY`, `RTD`, `TRIMRANGE`

### Lógicas (4)

`BYCOL`, `BYROW`, `MAKEARRAY`, `SCAN`

### Informação (6)

`CELL`, `INFO`, `ISFORMULA`, `ISOMITTED`, `SHEET`, `SHEETS`

### Matemáticas (0)

### Estatísticas (6)

`FORECAST.ETS`, `FORECAST.ETS.CONFINT`, `FORECAST.ETS.SEASONALITY`, 
`FORECAST.ETS.STAT`, `LINEST`, `LOGEST`

### Compatibilidade (0)

### Financeiras (18)

`ACCRINT`, `AMORDEGRC`, `AMORLINC`, `COUPDAYBS`, `COUPDAYS`, `COUPDAYSNC`, 
`COUPNCD`, `COUPNUM`, `COUPPCD`, `DURATION`, `MDURATION`, `ODDFPRICE`, 
`ODDFYIELD`, `ODDLPRICE`, `ODDLYIELD`, `PRICE`, `STOCKHISTORY`, `YIELD`

### Engenharia (2)

`BESSELK`, `BESSELY`

### Texto (6)

`ASC`, `BAHTTEXT`, `DBCS`, `DETECTLANGUAGE`, `PHONETIC`, `TRANSLATE`

### Cubo (7)

`CUBEKPIMEMBER`, `CUBEMEMBER`, `CUBEMEMBERPROPERTY`, `CUBERANKEDMEMBER`, 
`CUBESET`, `CUBESETCOUNT`, `CUBEVALUE`

### Web (2)

`FILTERXML`, `WEBSERVICE`

### Suplementos (3)

`CALL`, `EUROCONVERT`, `REGISTER.ID`

## Lote de 15/10/2026 (48 funções, em `fn_extra.rs`, `fn_conversao.rs` e `fn_titulos.rs`)

Todas com nome em português e em inglês (tabela de apelidos de `functions.rs`) e ajuda em
`function_help_data.rs`.

| Categoria | en / pt |
|---|---|
| Matriz dinâmica (13) | VSTACK/EMPILHARV, HSTACK/EMPILHARH, TAKE/PEGAR, DROP/DESCARTAR, CHOOSECOLS/ESCOLHERCOLS, CHOOSEROWS/ESCOLHERLINS, TOCOL/PARACOL, TOROW/PARALIN, WRAPROWS/QUEBRARLINS, WRAPCOLS/QUEBRARCOLS, EXPAND/EXPANDIR, SORTBY/CLASSIFICARPOR, RANDARRAY/MATRIZALEATÓRIA |
| Pesquisa e informação (5) | LOOKUP/PROC, XMATCH/CORRESPX, ADDRESS/ENDEREÇO, HYPERLINK/HIPERLINK, ISREF/ÉREF |
| Estatística (8) | STEYX/EPADYX, PROB/PROB, F.TEST/TESTE.F, Z.TEST/TESTE.Z, TREND/TENDÊNCIA, GROWTH/CRESCIMENTO, e os nomes de compatibilidade FTEST/TESTEF, ZTEST/TESTEZ |
| Texto (8) | LENB/NÚM.CARACTB, LEFTB/ESQUERDAB, RIGHTB/DIREITAB, MIDB/EXT.TEXTOB, FINDB/PROCURARB, SEARCHB/LOCALIZARB, REPLACEB/MUDARB, ENCODEURL/CODIFURL |
| Engenharia (3) | CONVERT/CONVERTER, BESSELJ/BESSELJ, BESSELI/BESSELI |
| Financeiras (11) | DISC/DESC, PRICEDISC/PREÇODESC, YIELDDISC/LUCRODESC, INTRATE/TAXAJUROS, RECEIVED/RECEBER, TBILLPRICE/OTNVALOR, TBILLYIELD/OTNLUCRO, TBILLEQ/OTN, PRICEMAT/PREÇOVENC, YIELDMAT/LUCROVENC, ACCRINTM/JUROSACUMV |

Limites conhecidos: TENDÊNCIA/CRESCIMENTO aceitam uma só variável X (várias colunas de X dão
`#VALOR!`); CORRESPX trata os modos de pesquisa binária (2 e -2) como pesquisa linear; PROC
supõe o vetor em ordem crescente; as funções `*B` valem como as sem B (só idiomas de byte duplo
contam bytes); HIPERLINK devolve o texto, abrir o vínculo é da interface; ÉREF não reconhece
nome definido como referência.

## Conferência com a página oficial (14/10/2026)

Comparada a lista acima com a página da Microsoft "Funções do Excel (por categoria)"
(pt-BR e en-US): **520 funções; o motor tem 409; as 111 que faltam estão todas nesta
lista** (nenhuma falta fora dela; contagem de 14/10, antes do lote de 15/10). O motor tem ainda 7 fora da página (COUNTUNIQUE,
FETCHTEXT, IMAGE.BASE64, IMPORTDATA, IMPORTJSON, JOIN, REGEXMATCH).

### Funções que temos sem o nome em português

Regra do usuário: com a interface em português, a fórmula aparece só em português. As
funções abaixo existem no motor, mas sem apelido pt-BR. **Cuidado:** a coluna com o nome
em português vem da página da Microsoft, que é tradução automática e erra vários nomes
(ex.: POWER aparece como "ENERGIA", o Excel usa POTÊNCIA; COLUMN como "COLUNA", o Excel
usa COL; TIME como "HORÁRIO", o Excel usa TEMPO; SWITCH como "MUDAR", o Excel usa
PARÂMETRO). Conferir cada nome com o Excel em português antes de criar o apelido.

| Função (en) | Nome pt-BR na página | Apelidos pt no motor |
|---|---|---|
| CHIDIST | QUIDIST | - |
| CHIINV | QUIINV | INV.QUI |
| CHITEST | QUITEST | - |
| CONFIDENCE | CONFIANÇA | INT.CONFIANCA, INT.CONFIANÇA |
| RANK | CLASSIFICAÇÃO | ORDEM |
| CONCATENATE | CONCATENAR | - |
| TIME | HORÁRIO | TEMPO |
| WORKDAY.INTL | DIA DE TRABALHO.INTL | DIATRABALHO.INTL |
| BITLSHIFT | DESLOCAÇÃO RÍGIDA | DESLOCESQBIT |
| BITRSHIFT | DESLOCAÇÃO DE BITS | DESLOCDIRBIT |
| ERF.PRECISE | ERF.PRECISO | FUNERRO.PRECISO |
| ERFC.PRECISE | FUNERRO.PRECISO | FUNERROCOMPL.PRECISO |
| PDURATION | DURAÇÃO PA | DURACAOP, DURAÇÃOP |
| ISBLANK | ÉCÉL.VAZIA | ÉCÉL |
| ISERR | ÉERRO | EERR, ÉERR |
| ISERROR | ÉERROS | É.ERRO, ÉERRO |
| ISNUMBER | ÉNÚMERO | ÉNÚM |
| NA | N/D | NA, NAO.DISP, NAODISP, NÃO.DISP, NÃODISP |
| MAP | MAPA | MAPEAR |
| SWITCH | MUDAR | PARAMETRO, PARÂMETRO |
| CHOOSE | ESCOLHA | ESCOLHER |
| COLUMN | COLUNA | COL |
| COLUMNS | COLUNAS | COLS |
| FILTER | FILTRAR | FILTRO |
| ROW | LINHA | LIN |
| AGGREGATE | AGREGADO | AGREGAR |
| ARABIC | ÁRABE | ARABICO, ARÁBICO |
| CEILING | TETO | - |
| CEILING.MATH | TETO.MATEMÁTICA | TETO.MAT |
| COMBIN | COMBINAR | COMBIN, COMBINAÇÃO |
| FLOOR.MATH | CHÃO.MATEMÁTICA | ARREDMULTB.MAT |
| PERCENTOF | PORCENTAGEM DE | - |
| POWER | ENERGIA | POTENCIA, POTÊNCIA |
| PRODUCT | PRODUTO | - |
| AVERAGEIF | MÉDIAIF | MEDIASE, MÉDIASE |
| BETA.DIST | VERSÃO BETA.DIST | DIST.BETA.N |
| BETA.INV | VERSÃO BETA.INV. | INV.BETA.N |
| BINOM.DIST.RANGE | BINOM.DIST.INTERVALO | DIST.BINOM.INTERVALO |
| BINOM.INV | BINOM.INV. | INV.BINOM |
| CHISQ.INV | CHISQ.INV. | INV.QUIQUA |
| CHISQ.TEST | CHISQ.TESTE | CHITEST, TESTE.QUI, TESTEQUI |
| CONFIDENCE.NORM | CONFIANÇA.NORMA | INT.CONFIANCA.NORM, INT.CONFIANÇA.NORM |
| CONFIDENCE.T | CONFIANÇA.T | INT.CONFIANCA.T, INT.CONFIANÇA.T |
| GAMMA.DIST | GAMA.DIST | DIST.GAMA |
| GAMMA.INV | GAMA.INV. | INV.GAMA |
| LOGNORM.INV | LOGNORM.INV. | INV.LOGNORMAL.N |
| MODE.SNGL | MODO.ÚNICO | - |
| NORM.DIST | NORMA.DIST | DIST.NORM, DISTNORM |
| NORM.INV | NORMA.INV. | INV.NORM, INVNORM |
| NORM.S.DIST | NORMA.S.DIST | DIST.NORMP, DISTNORMP |
| NORM.S.INV | NORMA.S.INV | INV.NORMP.N |
| PERCENTILE.INC | PERCENTIL.INC | - |
| PERCENTRANK.EXC | CLASSIFICAÇÃO PERCENTUAL.EXC | ORDEM.PORCENTUAL.EXC |
| PERCENTRANK.INC | CLASSIFICAÇÃO PERCENTUAL.INC | ORDEM.PORCENTUAL.INC |
| PERMUTATIONA | PERMUTAS | - |
| QUARTILE.INC | QUARTIL.INC | - |
| RANK.AVG | CLASSIFICAÇÃO.MÉD | ORDEM.MED, ORDEM.MÉD |
| STDEV.S | DESVPAD.A | - |
| VAR.S | VAR.A | - |
| CLEAN | LIMPAR | TIRAR |
| DOLLAR | DÓLAR | MOEDA |
| FIND | PROCURAR | LOCALIZAR |
| FIXED | CORRIGIDO | DEF.NUM.DEC, DEF.NÚM.DEC |
| REGEXEXTRACT | EXTRATO REGEX | REGEXEXTRAIR |
| REGEXTEST | TESTE DE REGISTRO | REGEXTESTE |
| SEARCH | LOCALIZAR | PROCURAR |
| UNICHAR | CARACT UNICHAR | CARACTUNICODE |
