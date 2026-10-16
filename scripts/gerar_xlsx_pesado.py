"""Gera um .xlsx pesado, com milhões de fórmulas, sem bibliotecas externas.

Uso: python3 scripts/gerar_xlsx_pesado.py SAIDA.xlsx [LINHAS]  (padrão: 1.000.000 de linhas)
O XML de cada aba é escrito em fluxo dentro do ZIP: a memória fica baixa
mesmo com 1 milhão de linhas.
"""
import random
import sys
import time
import zipfile
from xml.sax.saxutils import escape

SAIDA = sys.argv[1]
N = int(sys.argv[2]) if len(sys.argv) > 2 else 1_000_000
ULT = N + 1  # última linha de dados (linha 1 é o cabeçalho)

REGIOES = ["Norte", "Nordeste", "Centro-Oeste", "Sudeste", "Sul", "Litoral", "Interior", "Capital"]
CATEGORIAS = ["Eletrônicos", "Casa", "Escritório", "Esporte", "Brinquedos", "Alimentos", "Beleza", "Ferramentas"]
PRODUTOS = [(f"P{i:03d}", f"Produto {i:03d}", CATEGORIAS[i % len(CATEGORIAS)], round(5 + (i * 37 % 495) + 0.99, 2))
            for i in range(1, 201)]
random.seed(42)


def col(c):
    s = ""
    c += 1
    while c:
        c, r = divmod(c - 1, 26)
        s = chr(65 + r) + s
    return s


def num(ref, v, estilo=0):
    s = f' s="{estilo}"' if estilo else ""
    return f'<c r="{ref}"{s}><v>{v}</v></c>'


def txt(ref, v, estilo=0):
    s = f' s="{estilo}"' if estilo else ""
    return f'<c r="{ref}"{s} t="inlineStr"><is><t>{escape(v)}</t></is></c>'


def fml(ref, f, estilo=0, texto=False):
    s = f' s="{estilo}"' if estilo else ""
    t = ' t="str"' if texto else ""
    return f'<c r="{ref}"{s}{t}><f>{escape(f)}</f></c>'


def linha(r, celulas):
    return f'<row r="{r}">' + "".join(celulas) + "</row>"


CABECA = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
          '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
          '<sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" '
          'activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><sheetData>')
PE = "</sheetData></worksheet>"

DATA, MOEDA = 1, 2  # índices de estilo (styles.xml abaixo)


def aba_dados(saida):
    saida.write(CABECA.encode())
    titulos = ["ID", "Data", "Região", "Produto", "Qtd", "Preço", "Total", "Desconto", "Líquido",
               "Mês", "Acumulado", "Categoria", "Faixa"]
    saida.write(linha(1, [txt(f"{col(i)}1", t) for i, t in enumerate(titulos)]).encode())
    base = 46023  # 01/01/2026
    lote = []
    inicio = time.time()
    for i in range(N):
        r = i + 2
        p = random.randrange(200)
        cel = [
            num(f"A{r}", i + 1),
            num(f"B{r}", base + random.randrange(365), DATA),
            txt(f"C{r}", REGIOES[random.randrange(8)]),
            txt(f"D{r}", PRODUTOS[p][0]),
            num(f"E{r}", random.randint(1, 100)),
            num(f"F{r}", PRODUTOS[p][3], MOEDA),
            fml(f"G{r}", f"E{r}*F{r}", MOEDA),
            fml(f"H{r}", f"IF(E{r}>50,G{r}*0.1,0)", MOEDA),
            fml(f"I{r}", f"G{r}-H{r}", MOEDA),
            fml(f"J{r}", f"MONTH(B{r})"),
            # Corrente de dependência: cada linha espera a anterior.
            fml(f"K{r}", f"I{r}" if r == 2 else f"K{r - 1}+I{r}", MOEDA),
            fml(f"L{r}", f"_xlfn.XLOOKUP(D{r},Produtos!$A$2:$A$201,Produtos!$C$2:$C$201)", texto=True),
            fml(f"M{r}", f'IF(I{r}>2000,"Alto",IF(I{r}>800,"Médio","Baixo"))', texto=True),
        ]
        lote.append(linha(r, cel))
        if len(lote) == 5000:
            saida.write("".join(lote).encode())
            lote.clear()
            if r % 100_000 < 5000:
                print(f"  Dados: {r - 1:>9,} linhas · {time.time() - inicio:5.1f} s", flush=True)
    saida.write("".join(lote).encode())
    saida.write(PE.encode())


def aba_produtos(saida):
    saida.write(CABECA.encode())
    titulos = ["Código", "Nome", "Categoria", "Preço base", "Vendido (líquido)", "Pedidos", "Ticket médio"]
    saida.write(linha(1, [txt(f"{col(i)}1", t) for i, t in enumerate(titulos)]).encode())
    for i, (cod, nome, cat, preco) in enumerate(PRODUTOS):
        r = i + 2
        saida.write(linha(r, [
            txt(f"A{r}", cod), txt(f"B{r}", nome), txt(f"C{r}", cat), num(f"D{r}", preco, MOEDA),
            fml(f"E{r}", f"SUMIFS(Dados!$I$2:$I${ULT},Dados!$D$2:$D${ULT},A{r})", MOEDA),
            fml(f"F{r}", f"COUNTIFS(Dados!$D$2:$D${ULT},A{r})"),
            fml(f"G{r}", f"IF(F{r}=0,0,E{r}/F{r})", MOEDA),
        ]).encode())
    saida.write(PE.encode())


def aba_resumo(saida):
    saida.write(CABECA.encode())
    meses = ["Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez"]
    saida.write(linha(1, [txt("A1", "Líquido por região e mês")] +
                      [txt(f"{col(j + 1)}1", m) for j, m in enumerate(meses)] + [txt("N1", "Total")]).encode())
    r = 2
    for reg in REGIOES:
        cel = [txt(f"A{r}", reg)]
        for m in range(12):
            c = col(m + 1)
            cel.append(fml(f"{c}{r}", f"SUMIFS(Dados!$I$2:$I${ULT},Dados!$C$2:$C${ULT},$A{r},"
                                      f"Dados!$J$2:$J${ULT},{m + 1})", MOEDA))
        cel.append(fml(f"N{r}", f"SUM(B{r}:M{r})", MOEDA))
        saida.write(linha(r, cel).encode())
        r += 1
    r += 1
    saida.write(linha(r, [txt(f"A{r}", "Pedidos por região e mês")]).encode())
    r += 1
    for reg in REGIOES:
        cel = [txt(f"A{r}", reg)]
        for m in range(12):
            cel.append(fml(f"{col(m + 1)}{r}", f"COUNTIFS(Dados!$C$2:$C${ULT},$A{r},Dados!$J$2:$J${ULT},{m + 1})"))
        saida.write(linha(r, cel).encode())
        r += 1
    r += 1
    rng = lambda c: f"Dados!${c}$2:${c}${ULT}"
    indicadores = [
        ("Líquido total", f"SUM({rng('I')})"),
        ("Acumulado final (corrente)", f"Dados!K{ULT}"),
        ("Pedidos Alto no Sul (SUMPRODUCT)", f'SUMPRODUCT(({rng("C")}="Sul")*({rng("M")}="Alto"))'),
        ("Líquido Alto no Sudeste (SUMPRODUCT)", f'SUMPRODUCT(({rng("C")}="Sudeste")*({rng("M")}="Alto")*{rng("I")})'),
        ("Ticket médio geral", f"AVERAGE({rng('I')})"),
        ("Maior pedido", f"MAX({rng('I')})"),
        ("Média de Eletrônicos", f'AVERAGEIFS({rng("I")},{rng("L")},"Eletrônicos")'),
    ]
    for nome, f in indicadores:
        saida.write(linha(r, [txt(f"A{r}", nome), fml(f"B{r}", f, MOEDA)]).encode())
        r += 1
    saida.write(PE.encode())


ARQUIVOS_FIXOS = {
    "[Content_Types].xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/worksheets/sheet3.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>""",
    "_rels/.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>""",
    "xl/workbook.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="Resumo" sheetId="3" r:id="rId3"/>
<sheet name="Dados" sheetId="1" r:id="rId1"/>
<sheet name="Produtos" sheetId="2" r:id="rId2"/>
</sheets>
<calcPr calcId="191029" fullCalcOnLoad="1"/>
</workbook>""",
    "xl/_rels/workbook.xml.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet3.xml"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>""",
    "xl/styles.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<numFmts count="1"><numFmt numFmtId="164" formatCode="&quot;R$&quot; #,##0.00"/></numFmts>
<fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="3">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="14" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
</cellXfs>
</styleSheet>""",
}

inicio = time.time()
with zipfile.ZipFile(SAIDA, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
    for nome, conteudo in ARQUIVOS_FIXOS.items():
        z.writestr(nome, conteudo)
    for nome, gerar in [("xl/worksheets/sheet1.xml", aba_dados),
                        ("xl/worksheets/sheet2.xml", aba_produtos),
                        ("xl/worksheets/sheet3.xml", aba_resumo)]:
        with z.open(nome, "w", force_zip64=True) as saida:
            gerar(saida)
print(f"Pronto em {time.time() - inicio:.1f} s: {SAIDA}")
