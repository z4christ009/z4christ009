#!/usr/bin/env python3
"""Build the revised "Wounded Image" report as a formatted .docx and .pdf.

The report content is defined once in ``BLOCKS`` and rendered to both formats so
the two stay in sync. APA 7 student-paper style is approximated: Times New Roman
12pt, double spacing, 1-inch margins, a title page, an abstract, headings, and a
references list.
"""

RUNNING_HEAD = "THE WOUNDED IMAGE: ABBA AND EARTHLY FATHERHOOD"

TITLE = 'The Wounded Image: Reclaiming "Abba" as the True Face of God\u2019s Fatherhood'
AUTHOR = "Christina Elias"
AFFILIATION = "Notre Dame University\u2013Louaize (NDU)"
COURSE = "Marriage & Family in the Catholic Church \u2013 REG 314"
INSTRUCTOR = "Dr. Emilie Jibrine"
DATE = "July 4, 2026"

ABSTRACT = (
    "Many people who believe in God still find it hard to trust Him as a Father. "
    "This report addresses that difficulty using both theology and honest "
    "reflection on how the human heart actually works. Its center of gravity is "
    "one small Aramaic word: Abba, the intimate, unguarded way Jesus spoke to God, "
    "and the same word Paul says every believer is given the right to use "
    "(Romans 8:15). Our first experience of a father teaches us, long before we can "
    "put it into words, what kind of thing a father is: safe or unsafe, near or far, "
    "warm or cold. When that first experience is wounded, we tend to hand God the "
    "bill. This paper argues that the Trinity, the Incarnation, and the cross\u2014"
    "rather than our own history\u2014should have the last word on who God the Father "
    "actually is. It then turns to the marriage-and-family question at the heart of "
    "this course: because human fatherhood was designed to image divine fatherhood "
    "(Ephesians 3:14\u201315), a husband and father can consciously reflect the way "
    "God expresses His fatherhood, and in doing so he shapes both the faith of his "
    "family and the kind of man he himself becomes."
)

# Each block: ("h1"|"h2"|"body", text)
BLOCKS = [
    ("h1", "The Wounded Image: Reclaiming \u201cAbba\u201d as the True Face of God\u2019s Fatherhood"),

    ("h2", "Introduction"),
    ("body",
     "Most of us know a father before we know anything about God. Maybe he was "
     "there, maybe he wasn\u2019t, and either way we learned something from it. We "
     "learned it in small ways: whether the sound of his footsteps coming near "
     "meant safety or fear, and whether love was something we had to earn. So by "
     "the time anyone tells us God is our Father, that word is already loaded and "
     "sometimes carries its own weight. It comes with a face attached (Rizzuto, "
     "1979), and for a lot of people it is not a kind face."),
    ("body",
     "This report addresses that problem. Everything here hangs on one word Jesus "
     "used: Abba. It is how he talked to God (Mark 14:36), and Paul says the Spirit "
     "gives the same word to everyone God adopts (Romans 8:15; Galatians 4:6). The "
     "argument is easy to say but hard to actually live: healing a wounded picture "
     "of God the Father means learning to believe Abba even when your history says "
     "otherwise. Let us also be clear about blame. Scripture puts responsibility on "
     "the person who made the choice (James 1:13\u201314). A father who was absent "
     "or cruel is that way as a result of his own choosing."),

    ("h2", "The Biblical Standard: What God\u2019s Fatherhood Actually Is"),
    ("body",
     "Before we can say a father wounds the image of God, we have to know what that "
     "image is. God feels for us the way a good father feels for his children, and "
     "the comparison starts with God, not with us (Psalm 103:13, English Standard "
     "Version, 2001/2016). In fact the direction only runs one way: no one is a "
     "father the way God is a Father, because His fatherhood is the origin and the "
     "measure of every other kind (Catholic Church, 1997, no. 239). So when "
     "Scripture shows God fathering His people, it is not borrowing a nice human "
     "picture; it is showing us the original that every human father is a copy of."),
    ("body",
     "Read across the whole story, God expresses His fatherhood in a handful of "
     "recognizable ways, and it is worth naming them plainly because these are the "
     "exact things a wounded person doubts. First, He initiates. God moves before "
     "we do: the father in the parable spots his son far off and runs (Luke 15:20), "
     "and \u201cwe love because he first loved us\u201d (1 John 4:19). Second, He "
     "delights. God says He is pleased with His Son before that Son has done "
     "anything public (Matthew 3:17); the approval comes first, not as a reward for "
     "a finished performance. Third, He is present. He is not a distant "
     "administrator but the one who promises, \u201cI will never leave you nor "
     "forsake you\u201d (Hebrews 13:5). Fourth, He provides and protects, giving "
     "good gifts to His children the way a decent father would, only more so "
     "(Matthew 7:9\u201311). Fifth, He disciplines\u2014but the point of it is to "
     "form a child, not to crush one, and Scripture ties it directly to love: "
     "\u201cthe Lord disciplines the one he loves\u201d (Hebrews 12:6). Sixth, He "
     "restores; He puts the robe and the ring on a boy who had not even finished "
     "his apology (Luke 15:22). And underneath all of it, He gives Himself, "
     "spending His own Son rather than protecting Himself at our expense "
     "(Romans 8:32). Taken together, that is the standard: initiating, delighting, "
     "present, providing, forming, forgiving, self-giving love."),
    ("body",
     "Paul uses \u2018Abba\u2019 in the exact place where believers start feeling "
     "like hired help: \u201cyou did not receive the spirit of slavery to fall back "
     "into fear, but you have received the Spirit of adoption as sons, by whom we "
     "cry, \u2018Abba! Father!\u2019\u201d (Romans 8:15). It is not a nice detail on "
     "the side; it is the entire point."),

    ("h2", "Three Anchors Beneath the Word Abba"),

    ("h2", "The Trinity: Fatherhood Older Than the World"),
    ("body",
     "There were no human fathers yet when the Father was already loving the Son. "
     "Jesus says it plainly: \u201cyou loved me before the foundation of the "
     "world\u201d (John 17:24). That makes human fatherhood the copy, not the "
     "original. Our own fathers were working from an echo of something whole, and "
     "sometimes the echo reached them badly distorted. That says something about "
     "them. It says nothing about the sound they were copying."),

    ("h2", "The Incarnation: A Face We Were Not Left to Imagine"),
    ("body",
     "If all we had were arguments about God\u2019s character, a hurt person would "
     "fill in the blanks with the worst father he knows. That is what people who "
     "are hurting do. But Jesus told Philip, \u201cWhoever has seen me has seen the "
     "Father\u201d (John 14:9). So we can look at who he ate with, who he forgave, "
     "and what actually made him angry. We do not have to guess what God is like. "
     "We were shown."),

    ("h2", "The Atonement and Adoption: Abba as a Legal Fact"),
    ("body",
     "Calling God Abba is not just permitted; something had to happen first. God "
     "sent his Son \u201cto redeem those who were under the law, so that we might "
     "receive adoption as sons,\u201d and only after that does Paul say God sent the "
     "Spirit of his Son into our hearts, crying, \u201cAbba! Father!\u201d "
     "(Galatians 4:4\u20136). The adoption is already done. It does not wait for us "
     "to feel adopted, which means most of the healing is catching up emotionally "
     "to something that is already true on paper."),

    ("h2", "The Distortion: How Earthly Fathers Wound the Image of Abba"),
    ("body",
     "There is usually a gap between that picture and the one we grew up with. The "
     "prophets knew it, which is why they keep insisting that God is not like the "
     "fathers Israel actually had (Hosea 11:1\u20134)."),

    ("h2", "The Absent Father"),
    ("body",
     "A father who is gone, or who is home but not really there, teaches a child "
     "that people cannot be counted on. Years later that turns into a God who "
     "exists but never shows up (Sowers, 2010). Scripture answers that with a "
     "promise: \u201cMy father and my mother have forsaken me, but the Lord will "
     "take me in\u201d (Psalm 27:10)."),

    ("h2", "The Authoritarian or Harsh Father"),
    ("body",
     "A father who rules by control and hands out approval only when it is earned "
     "teaches another lesson: love is a wage, a payment for good behavior. Grow up "
     "with that and God becomes a scorekeeper\u2014which is strange, because the "
     "father in Luke 15 is running down a road."),

    ("h2", "The Abusive Father"),
    ("body",
     "When the father was the danger, closeness itself stops feeling safe. The "
     "very words Scripture uses to pull us toward God\u2014nearness, touch, "
     "submission, a father\u2019s authority\u2014are the same words that abuse has "
     "already ruined. It usually takes safe people, over years, to show that those "
     "words can mean something else, but most of all it takes seeing what the true "
     "fatherhood of God is actually like."),

    ("h2", "Where the Cosmic Conflict Fits\u2014and Where It Does Not"),
    ("body",
     "Scripture knew that our earliest relationships shape what we expect from love "
     "(Matthew 7:9\u201311). The wound belongs to the father who caused it (James "
     "1:13\u201314). What the cosmic conflict explains is the next part: there is an "
     "enemy whose whole strategy is lying about God\u2019s character (2 Corinthians "
     "4:4), and he has every reason to keep a hurt person from ever checking that "
     "lie against the real thing."),

    ("h2", "The Healing Pathway: Reclaiming God as Abba"),

    ("h2", "Theological Re-Parenting"),
    ("body",
     "Healing starts by meeting the God the Bible actually describes instead of the "
     "narrow version our personal history handed us. If God feels indifferent, "
     "there is the Shepherd who goes after one sheep (John 10:14). If God feels "
     "like a scorekeeper, there is the Warrior who \u201cfights for you\u201d "
     "(Deuteronomy 1:30). If God feels unfair, there is the Judge who gets it right "
     "(Isaiah 30:18)."),

    ("h2", "Church as Surrogate Family"),
    ("body",
     "Jesus said family is not only about blood: \u201cWhoever does the will of God, "
     "he is my brother and sister and mother\u201d (Mark 3:35). People who show up "
     "for each other, keep showing up, and do not disappear will slowly teach "
     "someone what safety feels like, and Paul\u2019s picture of older believers "
     "walking with younger ones shows the shape of it (Titus 2:1\u20138)."),

    ("h2", "Case Illustration"),
    ("body",
     "Theology sometimes does not reach the deepest part of a person. Brennan "
     "Manning (2015), a Catholic priest, grew up with an alcoholic father whose "
     "moods decided everything, and he says learning to call God Abba took most of "
     "his life. The distortion his own father had stamped onto the word \u201cGod\u201d "
     "took that long to undo. His story is a warning and a hope at once: a father "
     "can bend the image badly, and the image can still be straightened."),

    ("h2", "Reflecting Abba: The Husband and Father as a Living Image of God\u2019s Fatherhood"),
    ("body",
     "Everything above has one more thing to say, and it is the reason this belongs "
     "in a course on marriage and the family. If a wounded father can distort the "
     "word \u201cFather,\u201d then a healthy one can help repair it. Scripture says "
     "\u201cevery family in heaven and on earth\u201d is named after the Father "
     "(Ephesians 3:14\u201315), which means fatherhood on earth is not a human "
     "invention that we then project upward onto God; it is the other way around. "
     "The Church puts it as strongly as possible: in \u201crevealing and reliving "
     "on earth the very fatherhood of God,\u201d a man is called to secure the "
     "united growth of everyone in his family (John Paul II, 1981, no. 25). That is "
     "a staggering job description. The earthly father is not the source of that "
     "love; he is a mirror of it, and his whole task is to keep the mirror clean "
     "and pointed in the right direction."),
    ("body",
     "This also names something our own culture keeps missing. We are often told "
     "that ours is a society \u201cwithout fathers,\u201d and the absence is felt "
     "not as freedom but as a lack (Francis, 2016, no. 176). The answer is not "
     "fathers who dominate, and not fathers who vanish, but fathers who image "
     "Abba on purpose."),

    ("h2", "How a Father Reflects the Way God Loves"),
    ("body",
     "The reflection becomes concrete when we lay it beside the marks of God\u2019s "
     "fatherhood named earlier. God initiates; so a father pursues his children "
     "first, offering closeness before they have done anything to earn it, instead "
     "of waiting to be impressed. God delights; so a father blesses and affirms his "
     "child before the report card, the way \u201cthis is my beloved Son\u201d came "
     "before any public success (Matthew 3:17). God is present; so a father simply "
     "shows up, over and over, which is the direct answer to the wound the absent "
     "father leaves. God provides and protects, and a father does the ordinary, "
     "unglamorous work of shelter, food, and safety. God disciplines to form and "
     "not to crush, so Scripture warns fathers twice: \u201cdo not provoke your "
     "children to anger, but bring them up in the discipline and instruction of the "
     "Lord\u201d (Ephesians 6:4), and \u201cdo not provoke your children, lest they "
     "become discouraged\u201d (Colossians 3:21). God restores, so a father "
     "forgives and keeps the door open the way the father in Luke 15 did. And "
     "beneath all of it, God gives Himself\u2014which is why a husband is told to "
     "\u201clove your wives, as Christ loved the church and gave himself up for "
     "her\u201d (Ephesians 5:25). A father also passes on the faith itself, teaching "
     "his children the way of the Lord as part of daily life (Deuteronomy "
     "6:6\u20137). Put together, a father reflects Abba not by being perfect but by "
     "loving in the same shape God does: first, freely, faithfully, and at cost to "
     "himself."),

    ("h2", "What It Does to His Family"),
    ("body",
     "A father who loves this way is, whether he intends it or not, the first "
     "theology his children ever learn. Long before they can read a catechism, "
     "they are drawing conclusions about whether \u201cFather\u201d is a safe word, "
     "and their father is the main evidence. When his love initiates and does not "
     "have to be earned, the child grows up expecting God to be approachable rather "
     "than suspicious. When he is present, the child is far less likely to meet the "
     "\u201cGod who exists but never shows up.\u201d When he disciplines without "
     "cruelty, the child does not confuse authority with danger. In this way a home "
     "becomes a place where Abba is believable, and the wife is honored in the "
     "process, since a husband who images the self-giving of Christ toward her sets "
     "the emotional temperature of the whole house. The Church sees ordinary family "
     "life as exactly where this healing gets handed on from one generation to the "
     "next (Catholic Church, 1997, no. 2214). Most powerfully, a father who does "
     "this breaks a cycle. The distortion that gets passed down a family line "
     "(Malachi 4:6) can stop with one man who decides to reflect the original "
     "instead of repeating the damage he received."),

    ("h2", "What It Does to the Man Himself"),
    ("body",
     "None of this works if a father tries to be Abba on his own strength, because "
     "a mirror cannot generate light. He can only reflect what he has received, "
     "which means the first thing fatherhood asks of a man is that he learn to be a "
     "son\u2014that he let himself be loved by God before he is asked to love like "
     "God. This is quietly good news for the man himself. Fathering this way keeps "
     "confronting him with his own limits and his own wounds, and it becomes one of "
     "the main places he is sanctified: he has to forgive the way he wants to be "
     "forgiven, be patient past the end of his patience, and stay when leaving "
     "would be easier. It frees him, too. A man who knows he is already a beloved "
     "son does not need to run his home like a scorekeeper or prove his worth "
     "through his children\u2019s achievements. And there is a repair that runs "
     "backward: as he practices being the kind of father Abba is, his own "
     "distorted picture of God often starts to straighten, because he finally has "
     "a true experience of the thing the word was supposed to mean. In loving his "
     "family the way he was first loved, he becomes more himself, and more like "
     "Christ."),

    ("h2", "Conclusion"),
    ("body",
     "Fathers were supposed to be a small copy of something that starts in God, "
     "not the other way around (Ephesians 3:14\u201315). Sin cracked the copy. That "
     "is why the word \u201cFather\u201d leaves so many people uneasy instead of at "
     "peace. The question is which one we read through: the cracked copy, or the "
     "God who has already shown us what he is like."),
    ("body",
     "None of this undoes what happened. That was never the offer. The offer is "
     "that it does not get the final say, because the adoption was settled by "
     "someone else (Romans 8:15\u201317; Galatians 4:4\u20137). We get to say Abba "
     "anyway\u2014and the father who reflects Him gets to make that word believable "
     "for the people who live in his house."),

    ("h1", "References"),
    ("ref",
     "Catholic Church. (1997). Catechism of the Catholic Church (2nd ed.). Libreria "
     "Editrice Vaticana."),
    ("ref",
     "English Standard Version Bible. (2016). Crossway Bibles. (Original work "
     "published 2001)"),
    ("ref",
     "Francis. (2016). Amoris laetitia [Apostolic exhortation]. Libreria Editrice "
     "Vaticana."),
    ("ref",
     "John Paul II. (1981). Familiaris consortio [Apostolic exhortation]. Libreria "
     "Editrice Vaticana."),
    ("ref",
     "Manning, B. (2015). Abba\u2019s child: The cry of the heart for intimate "
     "belonging (Expanded ed.). NavPress."),
    ("ref",
     "Rizzuto, A.-M. (1979). The birth of the living God: A psychoanalytic study. "
     "University of Chicago Press."),
    ("ref",
     "Sowers, K. (2010). The fatherless generation: Redeeming the story. Wipf and "
     "Stock."),
]


def build_docx(path):
    from docx import Document
    from docx.shared import Pt, Inches
    from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_LINE_SPACING
    from docx.enum.section import WD_SECTION
    from docx.oxml.ns import qn
    from docx.oxml import OxmlElement

    doc = Document()

    # Base style: Times New Roman 12, double spaced.
    normal = doc.styles["Normal"]
    normal.font.name = "Times New Roman"
    normal.font.size = Pt(12)
    normal.element.rPr.rFonts.set(qn("w:eastAsia"), "Times New Roman")
    pf = normal.paragraph_format
    pf.line_spacing_rule = WD_LINE_SPACING.DOUBLE
    pf.space_after = Pt(0)
    pf.space_before = Pt(0)

    section = doc.sections[0]
    section.top_margin = Inches(1)
    section.bottom_margin = Inches(1)
    section.left_margin = Inches(1)
    section.right_margin = Inches(1)

    # Header: running head left, page number right.
    header = section.header
    hp = header.paragraphs[0]
    hp.text = RUNNING_HEAD + "\t\t"
    hp.alignment = WD_ALIGN_PARAGRAPH.LEFT
    for r in hp.runs:
        r.font.name = "Times New Roman"
        r.font.size = Pt(12)
    # page number field
    run = hp.add_run()
    fldStart = OxmlElement("w:fldChar")
    fldStart.set(qn("w:fldCharType"), "begin")
    instr = OxmlElement("w:instrText")
    instr.set(qn("xml:space"), "preserve")
    instr.text = "PAGE"
    fldEnd = OxmlElement("w:fldChar")
    fldEnd.set(qn("w:fldCharType"), "end")
    run._r.append(fldStart)
    run._r.append(instr)
    run._r.append(fldEnd)
    run.font.name = "Times New Roman"
    run.font.size = Pt(12)

    def add_body(text, first_indent=True):
        p = doc.add_paragraph()
        p.paragraph_format.line_spacing_rule = WD_LINE_SPACING.DOUBLE
        if first_indent:
            p.paragraph_format.first_line_indent = Inches(0.5)
        r = p.add_run(text)
        r.font.name = "Times New Roman"
        r.font.size = Pt(12)
        return p

    # ---- Title page ----
    for _ in range(3):
        add_body("", first_indent=False)
    for line, bold in [
        (TITLE, True),
        ("", False),
        (AUTHOR, False),
        (AFFILIATION, False),
        (COURSE, False),
        (INSTRUCTOR, False),
        (DATE, False),
    ]:
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        p.paragraph_format.line_spacing_rule = WD_LINE_SPACING.DOUBLE
        r = p.add_run(line)
        r.bold = bold
        r.font.name = "Times New Roman"
        r.font.size = Pt(12)

    doc.add_page_break()

    # ---- Abstract ----
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    p.paragraph_format.line_spacing_rule = WD_LINE_SPACING.DOUBLE
    r = p.add_run("Abstract")
    r.bold = True
    r.font.name = "Times New Roman"
    r.font.size = Pt(12)
    add_body(ABSTRACT, first_indent=False)
    doc.add_page_break()

    # ---- Body ----
    for kind, text in BLOCKS:
        if kind == "h1":
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
            p.paragraph_format.line_spacing_rule = WD_LINE_SPACING.DOUBLE
            r = p.add_run(text)
            r.bold = True
            r.font.name = "Times New Roman"
            r.font.size = Pt(12)
        elif kind == "h2":
            p = doc.add_paragraph()
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            p.paragraph_format.line_spacing_rule = WD_LINE_SPACING.DOUBLE
            r = p.add_run(text)
            r.bold = True
            r.font.name = "Times New Roman"
            r.font.size = Pt(12)
        elif kind == "ref":
            p = doc.add_paragraph()
            p.paragraph_format.line_spacing_rule = WD_LINE_SPACING.DOUBLE
            p.paragraph_format.left_indent = Inches(0.5)
            p.paragraph_format.first_line_indent = Inches(-0.5)
            r = p.add_run(text)
            r.font.name = "Times New Roman"
            r.font.size = Pt(12)
        else:
            add_body(text)

    doc.save(path)


def build_pdf(path):
    from reportlab.lib.pagesizes import letter
    from reportlab.lib.units import inch
    from reportlab.lib.enums import TA_CENTER, TA_LEFT
    from reportlab.platypus import (
        BaseDocTemplate, Frame, PageTemplate, Paragraph, Spacer, PageBreak,
    )
    from reportlab.lib.styles import ParagraphStyle
    from xml.sax.saxutils import escape

    LEADING = 24  # double spacing for 12pt

    body = ParagraphStyle(
        "body", fontName="Times-Roman", fontSize=12, leading=LEADING,
        firstLineIndent=0.5 * inch, alignment=TA_LEFT,
    )
    body_noindent = ParagraphStyle(
        "body_ni", parent=body, firstLineIndent=0,
    )
    center = ParagraphStyle(
        "center", fontName="Times-Roman", fontSize=12, leading=LEADING,
        alignment=TA_CENTER,
    )
    center_bold = ParagraphStyle(
        "center_bold", parent=center, fontName="Times-Bold",
    )
    h2 = ParagraphStyle(
        "h2", fontName="Times-Bold", fontSize=12, leading=LEADING,
        alignment=TA_LEFT,
    )
    ref = ParagraphStyle(
        "ref", fontName="Times-Roman", fontSize=12, leading=LEADING,
        leftIndent=0.5 * inch, firstLineIndent=-0.5 * inch,
    )

    def header(canvas, doc_):
        canvas.saveState()
        canvas.setFont("Times-Roman", 12)
        canvas.drawString(inch, letter[1] - 0.6 * inch, RUNNING_HEAD)
        canvas.drawRightString(letter[0] - inch, letter[1] - 0.6 * inch,
                               str(doc_.page))
        canvas.restoreState()

    doc = BaseDocTemplate(
        path, pagesize=letter,
        leftMargin=inch, rightMargin=inch, topMargin=inch, bottomMargin=inch,
    )
    frame = Frame(inch, inch, letter[0] - 2 * inch, letter[1] - 2 * inch,
                  id="main")
    doc.addPageTemplates([PageTemplate(id="all", frames=[frame],
                                       onPage=header)])

    def esc(t):
        return escape(t)

    story = []
    # Title page
    story.append(Spacer(1, 2.5 * inch))
    story.append(Paragraph("<b>" + esc(TITLE) + "</b>", center))
    story.append(Spacer(1, LEADING))
    for line in [AUTHOR, AFFILIATION, COURSE, INSTRUCTOR, DATE]:
        story.append(Paragraph(esc(line), center))
    story.append(PageBreak())

    # Abstract
    story.append(Paragraph("<b>Abstract</b>", center_bold))
    story.append(Paragraph(esc(ABSTRACT), body_noindent))
    story.append(PageBreak())

    for kind, text in BLOCKS:
        if kind == "h1":
            story.append(Paragraph("<b>" + esc(text) + "</b>", center_bold))
        elif kind == "h2":
            story.append(Paragraph("<b>" + esc(text) + "</b>", h2))
        elif kind == "ref":
            story.append(Paragraph(esc(text), ref))
        else:
            story.append(Paragraph(esc(text), body))

    doc.build(story)


if __name__ == "__main__":
    import os
    here = os.path.dirname(os.path.abspath(__file__))
    docx_path = os.path.join(here, "Wounded_Image_Report_revised.docx")
    pdf_path = os.path.join(here, "Wounded_Image_Report_revised.pdf")
    build_docx(docx_path)
    build_pdf(pdf_path)
    print("Wrote:", docx_path)
    print("Wrote:", pdf_path)
