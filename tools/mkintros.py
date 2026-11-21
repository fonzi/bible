# Build crates/bible-core/data/intros.json from the Wikipedia extracts plus a short
# traditional view (author and date as historically held), written for this app.
import json, sys, importlib.util, os
here = os.path.dirname(os.path.abspath(__file__))
W = {}
for n in ("intros_part1", "intros_part2", "intros_part3"):
    spec = importlib.util.spec_from_file_location(n, os.path.join(here, n + ".py"))
    m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); W.update(m.W)
for pair, key in (("Sam", ("1Sam", "2Sam")), ("Kgs", ("1Kgs", "2Kgs")), ("Chr", ("1Chr", "2Chr"))):
    for k in key: W[k] = W[pair]
    del W[pair]
T = {
 "Gen": "Moses, during the wilderness years (c. 1446–1406 BC).", "Exod": "Moses (c. 1446–1406 BC).", "Lev": "Moses, at Sinai (c. 1445 BC).",
 "Num": "Moses (c. 1406 BC).", "Deut": "Moses, on the plains of Moab (c. 1406 BC); the account of his death added afterwards.",
 "Josh": "Joshua, with later additions (c. 1375 BC).", "Judg": "Samuel (c. 1050 BC).", "Ruth": "Samuel (c. 1000 BC).",
 "1Sam": "Samuel, with Nathan and Gad (c. 930 BC).", "2Sam": "Nathan and Gad, from Samuel's records (c. 930 BC).",
 "1Kgs": "Jeremiah, from royal records (c. 560 BC).", "2Kgs": "Jeremiah, from royal records (c. 560 BC).",
 "1Chr": "Ezra (c. 450–430 BC).", "2Chr": "Ezra (c. 450–430 BC).", "Ezra": "Ezra (c. 440 BC).", "Neh": "Nehemiah, from his memoirs (c. 430 BC).",
 "Esth": "Mordecai or a Jewish writer in Persia (c. 460 BC).", "Job": "Unknown; some traditions name Moses. Set in the time of the patriarchs.",
 "Ps": "David and others, including Asaph, the sons of Korah, Solomon and Moses (c. 1440–450 BC).", "Prov": "Solomon chiefly, with Agur and Lemuel (c. 950–700 BC).",
 "Eccl": "Solomon, late in life (c. 935 BC).", "Song": "Solomon (c. 960 BC).", "Isa": "Isaiah son of Amoz (c. 740–680 BC).",
 "Jer": "Jeremiah, with his scribe Baruch (c. 627–580 BC).", "Lam": "Jeremiah, after Jerusalem fell (586 BC).", "Ezek": "Ezekiel, among the exiles (593–571 BC).",
 "Dan": "Daniel, in Babylon and Persia (c. 530 BC).", "Hos": "Hosea (c. 755–715 BC).", "Joel": "Joel son of Pethuel; dates proposed range from the 9th to the 5th century BC.",
 "Amos": "Amos of Tekoa (c. 760 BC).", "Obad": "Obadiah (c. 586 BC or earlier).", "Jonah": "Jonah son of Amittai (c. 785 BC).", "Mic": "Micah of Moresheth (c. 735–700 BC).",
 "Nah": "Nahum (c. 660–630 BC).", "Hab": "Habakkuk (c. 607 BC).", "Zeph": "Zephaniah, in Josiah's reign (c. 630 BC).", "Hag": "Haggai (520 BC).",
 "Zech": "Zechariah son of Berechiah (c. 520–480 BC).", "Mal": "Malachi (c. 430 BC).",
 "Matt": "The apostle Matthew (c. AD 50–70).", "Mark": "John Mark, recording Peter's preaching (c. AD 55–65).", "Luke": "Luke the physician, companion of Paul (c. AD 60).",
 "John": "The apostle John, son of Zebedee (c. AD 85–95).", "Acts": "Luke (c. AD 62).", "Rom": "Paul, from Corinth (c. AD 57).", "1Cor": "Paul, from Ephesus (c. AD 55).",
 "2Cor": "Paul, from Macedonia (c. AD 56).", "Gal": "Paul (c. AD 49–55).", "Eph": "Paul, in prison in Rome (c. AD 60–62).", "Phil": "Paul, in prison in Rome (c. AD 61).",
 "Col": "Paul, in prison in Rome (c. AD 60–62).", "1Thess": "Paul, from Corinth (c. AD 51).", "2Thess": "Paul, from Corinth (c. AD 51–52).",
 "1Tim": "Paul (c. AD 62–64).", "2Tim": "Paul, shortly before his death in Rome (c. AD 67).", "Titus": "Paul (c. AD 62–64).", "Phlm": "Paul, in prison (c. AD 60–62).",
 "Heb": "Unknown; traditionally Paul, with Barnabas, Apollos and Luke also proposed (before AD 70).", "Jas": "James, the brother of Jesus (c. AD 45–49).",
 "1Pet": "The apostle Peter, from Rome (c. AD 62–64).", "2Pet": "The apostle Peter, shortly before his death (c. AD 65–68).", "1John": "The apostle John (c. AD 85–95).",
 "2John": "The apostle John, \"the elder\" (c. AD 85–95).", "3John": "The apostle John, \"the elder\" (c. AD 85–95).", "Jude": "Jude, brother of James and of Jesus (c. AD 65–80).",
 "Rev": "The apostle John, on Patmos (c. AD 95).",
}
assert len(W) == 66 and set(W) == set(T), (len(W), set(T) ^ set(W))
out = {k: {"article": a, "text": t, "traditional": T[k]} for k, (a, t) in W.items()}
json.dump(out, open(sys.argv[1], "w"), ensure_ascii=False, indent=0)
print(len(out), "intros")
