import sys
import json
L = []
def e(lane, title, start, end=None, summary="", refs=(), approx=True, note=None):
    d = dict(lane=lane, title=title, start=start, summary=summary, refs=list(refs), approx=approx)
    if end is not None: d["end"] = end
    if note: d["note"] = note
    L.append(d)

# ---- eras (top band) ----
eras = [
 ("Patriarchs", -2166, -1876), ("Israel in Egypt", -1876, -1446), ("Wilderness and conquest", -1446, -1375),
 ("Judges", -1375, -1050), ("United kingdom", -1050, -931), ("Divided kingdom", -931, -586),
 ("Exile", -586, -538), ("Persian period", -538, -332), ("Greek period", -332, -167),
 ("Maccabees and Hasmoneans", -167, -63), ("Roman Judea", -63, 30), ("Apostolic age", 30, 100),
 ("Early church", 100, 313), ("Imperial church", 313, 590), ("Middle Ages", 590, 1500),
 ("Reformation", 1500, 1650), ("Modern era", 1650, 2025)]

# ---- Bible events (traditional chronology) ----
B = "bible"
e(B, "Call of Abraham", -2091, summary="Abram leaves Harran for Canaan at God's call, with the promise of land, offspring and blessing for all nations.", refs=["Gen.12.1-Gen.12.9"], note="Traditional chronology; many scholars place the patriarchs a few centuries later.")
e(B, "Jacob's family settles in Egypt", -1876, summary="Joseph, now a ruler in Egypt, brings Jacob and his sons to Goshen during the famine.", refs=["Gen.46.1-Gen.47.12"])
e(B, "The Exodus", -1446, summary="Moses leads Israel out of Egypt; Passover, the crossing of the sea and the covenant at Sinai.", refs=["Exod.12.1-Exod.15.21", "Exod.19.1-Exod.20.21"], note="Early date from 1 Kings 6:1. Many scholars prefer a 13th-century date (c. 1260 BC).")
e(B, "Entry into Canaan", -1406, summary="After forty years in the wilderness, Israel crosses the Jordan under Joshua.", refs=["Josh.3.1-Josh.4.24"])
e(B, "Period of the judges", -1375, -1050, summary="Cycles of unfaithfulness, oppression and deliverance under leaders such as Deborah, Gideon and Samson.", refs=["Judg.2.6-Judg.2.23"])
e(B, "Solomon's temple dedicated", -959, summary="Built over seven years and dedicated with Solomon's prayer.", refs=["1Kgs.6.1", "1Kgs.8.1-1Kgs.8.66"])
e(B, "The kingdom divides", -931, summary="The northern tribes reject Rehoboam and follow Jeroboam: Israel in the north, Judah in the south.", refs=["1Kgs.12.1-1Kgs.12.33"], approx=False)
e(B, "Fall of Samaria", -722, summary="Assyria takes Samaria and deports the northern kingdom.", refs=["2Kgs.17.1-2Kgs.17.41"], approx=False)
e(B, "Sennacherib besieges Jerusalem", -701, summary="Assyria ravages Judah's towns, but Jerusalem is spared in Hezekiah's reign. Sennacherib's own annals also record the campaign.", refs=["2Kgs.18.13-2Kgs.19.37", "Isa.36.1-Isa.37.38"], approx=False)
e(B, "Josiah's reform", -622, summary="The Book of the Law is found in the temple; Josiah renews the covenant and removes idols.", refs=["2Kgs.22.1-2Kgs.23.30"])
e(B, "First deportation to Babylon", -605, summary="Nebuchadnezzar takes Daniel and other young nobles to Babylon.", refs=["Dan.1.1-Dan.1.7"])
e(B, "Jehoiachin taken to Babylon", -597, summary="Nebuchadnezzar takes the king, the court and Ezekiel into exile and sets up Zedekiah.", refs=["2Kgs.24.10-2Kgs.24.17"], approx=False)
e(B, "Jerusalem and the temple destroyed", -586, summary="After a long siege Babylon burns the temple and deports much of Judah.", refs=["2Kgs.25.1-2Kgs.25.21", "Jer.52.1-Jer.52.30"], note="Some date it to 587 BC.")
e(B, "Cyrus's decree and first return", -538, summary="Cyrus allows the exiles to go home and rebuild the temple; Zerubbabel leads the first return.", refs=["Ezra.1.1-Ezra.2.70"])
e(B, "Second temple completed", -516, summary="Finished in the sixth year of Darius after encouragement from Haggai and Zechariah.", refs=["Ezra.6.13-Ezra.6.22"])
e(B, "Esther in Susa", -479, -473, summary="A Jewish queen of Persia saves her people from Haman's plot; the origin of Purim.", refs=["Esth.2.16-Esth.2.18", "Esth.9.20-Esth.9.32"])
e(B, "Ezra returns to Jerusalem", -458, summary="Ezra the scribe brings a second group and teaches the Law.", refs=["Ezra.7.1-Ezra.7.10"])
e(B, "Nehemiah rebuilds the walls", -445, summary="Jerusalem's walls are rebuilt in fifty-two days despite opposition.", refs=["Neh.2.1-Neh.2.20", "Neh.6.15-Neh.6.16"])
e(B, "Maccabean revolt", -167, -164, summary="Judas Maccabeus leads the revolt against Antiochus IV; the temple is cleansed and rededicated, remembered at Hanukkah.", refs=["1Macc.4.36-1Macc.4.59", "John.10.22"])
e(B, "Herod begins rebuilding the temple", -20, summary="A vast expansion of the second temple that was still under way in Jesus' day.", refs=["John.2.20"])
e(B, "Birth of Jesus", -5, summary="Born in Bethlehem in the days of Herod the Great, who died in 4 BC.", refs=["Matt.2.1-Matt.2.12", "Luke.2.1-Luke.2.20"], note="Usually dated 6–4 BC.")
e(B, "Ministry of Jesus", 27, 30, summary="Baptism by John, teaching and healing in Galilee and Judea, and the journey to Jerusalem.", refs=["Luke.3.1-Luke.3.23", "Mark.1.9-Mark.1.15"], note="Often dated AD 27–30 or 29–33.")
e(B, "Crucifixion and resurrection", 30, summary="Jesus is crucified under Pontius Pilate at Passover and rises on the third day.", refs=["Luke.23.1-Luke.24.53", "1Cor.15.3-1Cor.15.8"], note="AD 30 or 33.")
e(B, "Pentecost", 30, summary="The Holy Spirit is poured out; Peter preaches and about three thousand are baptised.", refs=["Acts.2.1-Acts.2.41"])
e(B, "Paul's conversion", 34, summary="On the road to Damascus, the persecutor Saul meets the risen Jesus.", refs=["Acts.9.1-Acts.9.19", "Gal.1.13-Gal.1.17"])
e(B, "Council of Jerusalem", 49, summary="The apostles and elders agree that Gentile believers need not be circumcised.", refs=["Acts.15.1-Acts.15.35"])
e(B, "Paul's missionary journeys", 46, 57, summary="Churches founded across Galatia, Macedonia, Greece and Asia.", refs=["Acts.13.1-Acts.14.28", "Acts.15.36-Acts.18.22", "Acts.18.23-Acts.21.16"])
e(B, "Paul in Rome", 60, 62, summary="Under house arrest, Paul preaches freely for two years.", refs=["Acts.28.16-Acts.28.31"])
e(B, "Revelation written", 95, summary="John's visions on Patmos, traditionally dated to the end of Domitian's reign.", refs=["Rev.1.9-Rev.1.11"], note="Some date it to the late 60s.")

# ---- Kings of Israel (Thiele) ----
K = "israel"
for t, s, en, r in [
 ("Saul", -1050, -1010, "1Sam.10.1"), ("David", -1010, -970, "2Sam.5.1-2Sam.5.5"), ("Solomon", -970, -931, "1Kgs.2.12"),
 ("Jeroboam I", -931, -910, "1Kgs.12.20"), ("Nadab", -910, -909, "1Kgs.15.25"), ("Baasha", -909, -886, "1Kgs.15.33"),
 ("Elah", -886, -885, "1Kgs.16.8"), ("Zimri", -885, -885, "1Kgs.16.15"), ("Omri", -885, -874, "1Kgs.16.23"),
 ("Ahab", -874, -853, "1Kgs.16.29"), ("Ahaziah", -853, -852, "1Kgs.22.51"), ("Joram", -852, -841, "2Kgs.3.1"),
 ("Jehu", -841, -814, "2Kgs.9.1-2Kgs.10.36"), ("Jehoahaz", -814, -798, "2Kgs.13.1"), ("Jehoash", -798, -782, "2Kgs.13.10"),
 ("Jeroboam II", -793, -753, "2Kgs.14.23"), ("Zechariah", -753, -753, "2Kgs.15.8"), ("Shallum", -752, -752, "2Kgs.15.13"),
 ("Menahem", -752, -742, "2Kgs.15.17"), ("Pekahiah", -742, -740, "2Kgs.15.23"), ("Pekah", -752, -732, "2Kgs.15.27"),
 ("Hoshea", -732, -722, "2Kgs.17.1")]:
    lane = "united" if t in ("Saul", "David", "Solomon") else K
    e(lane, t, s, en, summary=("King over all Israel." if lane == "united" else "King of Israel (the northern kingdom)."), refs=[r], approx=(lane == "united"))
J = "judah"
for t, s, en, r in [
 ("Rehoboam", -931, -913, "1Kgs.14.21"), ("Abijah", -913, -911, "1Kgs.15.1"), ("Asa", -911, -870, "1Kgs.15.9"),
 ("Jehoshaphat", -872, -848, "1Kgs.22.41"), ("Jehoram", -853, -841, "2Kgs.8.16"), ("Ahaziah", -841, -841, "2Kgs.8.25"),
 ("Athaliah", -841, -835, "2Kgs.11.1-2Kgs.11.3"), ("Joash", -835, -796, "2Kgs.12.1"), ("Amaziah", -796, -767, "2Kgs.14.1"),
 ("Uzziah", -792, -740, "2Kgs.15.1"), ("Jotham", -750, -735, "2Kgs.15.32"), ("Ahaz", -735, -715, "2Kgs.16.1"),
 ("Hezekiah", -715, -686, "2Kgs.18.1"), ("Manasseh", -697, -642, "2Kgs.21.1"), ("Amon", -642, -640, "2Kgs.21.19"),
 ("Josiah", -640, -609, "2Kgs.22.1"), ("Jehoahaz", -609, -609, "2Kgs.23.31"), ("Jehoiakim", -609, -598, "2Kgs.23.36"),
 ("Jehoiachin", -598, -597, "2Kgs.24.8"), ("Zedekiah", -597, -586, "2Kgs.24.18")]:
    e(J, t, s, en, summary="King of Judah (the southern kingdom).", refs=[r], approx=False)

# ---- Prophets ----
P = "prophets"
for t, s, en, r, sm in [
 ("Samuel", -1100, -1015, "1Sam.3.1-1Sam.3.21", "Last of the judges; anoints Saul and David."),
 ("Nathan", -1000, -970, "2Sam.12.1-2Sam.12.15", "Confronts David after Bathsheba."),
 ("Elijah", -875, -848, "1Kgs.17.1", "Challenges Ahab and the prophets of Baal on Carmel."),
 ("Elisha", -848, -797, "2Kgs.2.1-2Kgs.2.15", "Succeeds Elijah; many miracles in Israel."),
 ("Jonah", -785, -775, "Jonah.1.1-Jonah.1.3", "Sent to preach to Nineveh."),
 ("Amos", -760, -750, "Amos.1.1", "A shepherd from Tekoa denounces injustice in Israel."),
 ("Hosea", -755, -715, "Hos.1.1", "Pictures God's faithful love through his own marriage."),
 ("Isaiah", -740, -681, "Isa.6.1-Isa.6.13", "Prophet in Jerusalem under Uzziah, Jotham, Ahaz and Hezekiah."),
 ("Micah", -735, -700, "Mic.1.1", "Foretells a ruler from Bethlehem."),
 ("Nahum", -660, -630, "Nah.1.1", "Announces Nineveh's fall."),
 ("Zephaniah", -640, -621, "Zeph.1.1", "Prophesies in Josiah's day."),
 ("Jeremiah", -627, -586, "Jer.1.1-Jer.1.10", "Warns Judah for forty years and promises a new covenant."),
 ("Habakkuk", -609, -598, "Hab.1.1-Hab.1.6", "Asks how God can use Babylon; 'the righteous shall live by faith'."),
 ("Daniel", -605, -536, "Dan.1.1-Dan.1.21", "Serves in the Babylonian and Persian courts."),
 ("Ezekiel", -593, -571, "Ezek.1.1-Ezek.1.3", "Visions among the exiles by the Chebar canal."),
 ("Haggai", -520, -520, "Hag.1.1", "Urges the returned exiles to rebuild the temple."),
 ("Zechariah", -520, -518, "Zech.1.1", "Visions of restoration and a coming king."),
 ("Malachi", -440, -420, "Mal.1.1", "The last Old Testament prophet."),
 ("John the Baptist", 28, 29, "Luke.3.1-Luke.3.20", "Preaches repentance and baptises Jesus.")]:
    e(P, t, s, en, summary=sm, refs=[r])

# ---- Powers ----
W = "powers"
for t, s, en, sm, r in [
 ("Egypt: Middle Kingdom", -2055, -1650, "A reunited Egypt; trade and building at home and abroad.", []),
 ("Old Babylonian kingdom", -1894, -1595, "Amorite dynasty of Babylon; Hammurabi unifies Mesopotamia.", []),
 ("Hittite empire", -1650, -1180, "Anatolian power that rivalled Egypt; fought at Kadesh in 1274 BC.", ["Josh.1.4"]),
 ("Egypt: New Kingdom", -1550, -1070, "Egypt's imperial age: Thutmose III, Akhenaten, Ramesses II.", []),
 ("Philistine city-states", -1175, -604, "Sea Peoples settled on the coast: Gaza, Ashkelon, Ashdod, Ekron, Gath.", ["1Sam.4.1-1Sam.4.11"]),
 ("Aramean kingdom of Damascus", -1000, -732, "Israel's frequent northern rival until Assyria took Damascus.", ["1Kgs.20.1"]),
 ("Neo-Assyrian empire", -911, -609, "Dominates the Near East; destroys Samaria and besieges Jerusalem.", ["2Kgs.15.19-2Kgs.15.20"]),
 ("Neo-Babylonian empire", -626, -539, "Nebuchadnezzar's Babylon conquers Judah.", ["2Kgs.24.1"]),
 ("Persian (Achaemenid) empire", -550, -330, "From Cyrus to Darius III; allows the exiles to return.", ["Ezra.1.1"]),
 ("Alexander the Great", -336, -323, "Conquers Persia, Egypt and the east; Greek language and culture spread.", ["Dan.8.5-Dan.8.8"]),
 ("Ptolemaic rule over Judea", -301, -198, "Judea governed from Alexandria; the Septuagint is translated there.", []),
 ("Seleucid rule over Judea", -198, -140, "Ruled from Antioch; Antiochus IV bans Jewish practice.", ["Dan.11.21-Dan.11.35"]),
 ("Hasmonean kingdom", -140, -37, "Independent Jewish state under the Maccabees' heirs.", []),
 ("Roman Republic in the east", -63, -27, "Pompey takes Jerusalem in 63 BC.", []),
 ("Herod the Great", -37, -4, "Rome's client king of Judea; massive building projects.", ["Matt.2.1-Matt.2.18"]),
 ("Roman Empire", -27, 476, "From Augustus to the fall of the West.", ["Luke.2.1"]),
 ("Byzantine (Eastern Roman) empire", 330, 1453, "Rome's eastern half, ruled from Constantinople.", []),
 ("Islamic caliphates", 632, 1258, "Rapid expansion across the Near East, North Africa and Spain.", []),
 ("Ottoman empire", 1299, 1922, "Rules the Holy Land from 1517 to 1917.", [])]:
    e(W, t, s, en, summary=sm, refs=r)
R = "rulers"
for t, s, en, sm, r in [
 ("Hammurabi", -1792, -1750, "King of Babylon, known for his law code.", []),
 ("Thutmose III", -1479, -1425, "Egypt's great warrior pharaoh; some link him with the oppression in Exodus.", []),
 ("Ramesses II", -1279, -1213, "Long-reigning pharaoh; often linked with the Exodus by those who hold a late date.", []),
 ("Shishak (Shoshenq I)", -943, -922, "Pharaoh who raided Judah after Solomon.", ["1Kgs.14.25-1Kgs.14.26"]),
 ("Tiglath-pileser III", -745, -727, "Assyrian king who made Israel and Judah pay tribute.", ["2Kgs.15.29", "2Kgs.16.7-2Kgs.16.10"]),
 ("Sargon II", -722, -705, "Completed the deportation of Israel.", ["Isa.20.1"]),
 ("Sennacherib", -705, -681, "Besieged Jerusalem under Hezekiah.", ["2Kgs.18.13"]),
 ("Nebuchadnezzar II", -605, -562, "Captured Jerusalem and destroyed the temple.", ["Dan.1.1", "2Kgs.25.1"]),
 ("Cyrus the Great", -559, -530, "Founder of the Persian empire; freed the exiles.", ["Isa.45.1", "Ezra.1.1-Ezra.1.4"]),
 ("Darius I", -522, -486, "Confirmed the temple decree.", ["Ezra.6.1-Ezra.6.15"]),
 ("Xerxes I (Ahasuerus)", -486, -465, "The king of Esther.", ["Esth.1.1"]),
 ("Artaxerxes I", -465, -424, "Sent Ezra and Nehemiah.", ["Ezra.7.11-Ezra.7.26", "Neh.2.1-Neh.2.8"]),
 ("Antiochus IV Epiphanes", -175, -164, "Desecrated the temple in 167 BC.", ["1Macc.1.20-1Macc.1.64"]),
 ("Augustus", -27, 14, "First Roman emperor; ordered the census of Luke 2.", ["Luke.2.1"]),
 ("Tiberius", 14, 37, "Emperor during Jesus' ministry.", ["Luke.3.1"]),
 ("Pontius Pilate", 26, 36, "Prefect of Judea who sentenced Jesus.", ["Matt.27.1-Matt.27.26"]),
 ("Claudius", 41, 54, "Expelled Jews from Rome.", ["Acts.18.2"]),
 ("Nero", 54, 68, "Persecuted Christians after the fire of AD 64; Paul and Peter martyred by tradition.", []),
 ("Vespasian and Titus", 69, 81, "Crushed the Jewish revolt and destroyed Jerusalem.", []),
 ("Domitian", 81, 96, "Emperor at the traditional date of Revelation.", []),
 ("Trajan", 98, 117, "Corresponded with Pliny about Christians.", []),
 ("Diocletian", 284, 305, "Launched the Great Persecution.", []),
 ("Constantine", 306, 337, "First Christian emperor; called the Council of Nicaea.", []),
 ("Theodosius I", 379, 395, "Made Nicene Christianity the state religion.", []),
 ("Justinian I", 527, 565, "Built Hagia Sophia; codified Roman law.", []),
 ("Charlemagne", 768, 814, "Crowned emperor in Rome in AD 800.", []),
 ("Henry VIII", 1509, 1547, "Broke with Rome; head of the Church of England.", []),
 ("Charles V", 1519, 1556, "Holy Roman Emperor at the Diet of Worms.", [])]:
    e(R, t, s, en, summary=sm, refs=r)

# ---- World and culture ----
C = "world"
for t, s, en, sm, ap in [
 ("Code of Hammurabi", -1754, None, "One of the oldest written law codes, carved on a basalt stele.", True),
 ("Late Bronze Age collapse", -1177, None, "Empires fall across the eastern Mediterranean; the Sea Peoples appear.", True),
 ("Phoenician alphabet", -1050, None, "Ancestor of the Hebrew, Greek and Latin alphabets.", True),
 ("First Olympic Games", -776, None, "Held at Olympia in Greece.", False),
 ("Founding of Rome", -753, None, "The traditional date.", False),
 ("Homer's epics", -750, None, "The Iliad and the Odyssey take written form.", True),
 ("Library of Ashurbanipal", -668, -627, "Thousands of clay tablets at Nineveh, including the Gilgamesh epic.", True),
 ("Siddhartha Gautama (the Buddha)", -563, -483, "Traditional dates; many scholars place him about a century later.", True),
 ("Confucius", -551, -479, "Chinese teacher whose sayings shaped East Asian thought.", True),
 ("Roman Republic founded", -509, None, "Rome expels its last king.", True),
 ("Parthenon built", -447, -432, "The temple of Athena on the Athenian Acropolis.", False),
 ("Death of Socrates", -399, None, "The Athenian philosopher is executed.", False),
 ("Septuagint begun", -250, None, "The Hebrew Scriptures translated into Greek at Alexandria.", True),
 ("Qin unifies China", -221, None, "Qin Shi Huang becomes the first emperor of China.", False),
 ("Dead Sea Scrolls copied", -250, 68, "The Qumran scrolls, the oldest surviving Hebrew Bible manuscripts.", True),
 ("Julius Caesar assassinated", -44, None, "The end of the Roman Republic draws near.", False),
 ("Great Fire of Rome", 64, None, "Nero blames the Christians.", False),
 ("Destruction of the Second Temple", 70, None, "Roman legions under Titus destroy Jerusalem.", False),
 ("Pompeii destroyed", 79, None, "Vesuvius buries Pompeii and Herculaneum.", False),
 ("Fall of the Western Roman Empire", 476, None, "The last Western emperor is deposed.", False),
 ("Muhammad's Hijra", 622, None, "Beginning of the Islamic calendar.", False),
 ("Gutenberg's printed Bible", 1455, None, "Movable-type printing transforms the spread of books.", True),
 ("Fall of Constantinople", 1453, None, "The Ottomans take the Byzantine capital.", False),
 ("Columbus reaches the Americas", 1492, None, "The first of four voyages.", False)]:
    e(C, t, s, en, summary=sm, approx=ap)

# ---- Church history ----
H = "church"
for t, s, en, sm, ap in [
 ("Clement of Rome writes to Corinth", 96, None, "One of the earliest Christian writings outside the New Testament.", True),
 ("Ignatius of Antioch martyred", 108, None, "Writes seven letters on his way to death in Rome.", True),
 ("Polycarp martyred", 155, None, "Bishop of Smyrna and a disciple of John.", True),
 ("Justin Martyr's First Apology", 155, None, "Describes Christian worship and defends the faith to the emperor.", True),
 ("Irenaeus, Against Heresies", 180, None, "A refutation of Gnosticism that appeals to the apostolic tradition.", True),
 ("Tertullian", 197, 220, "First major Latin theologian; coins the term 'Trinity' (trinitas).", True),
 ("Origen", 185, 253, "Alexandrian scholar; the Hexapla and On First Principles.", True),
 ("Decian persecution", 250, 251, "The first empire-wide persecution.", False),
 ("Great Persecution", 303, 311, "Diocletian orders churches and Scriptures destroyed.", False),
 ("Edict of Milan", 313, None, "Constantine and Licinius grant religious toleration.", False),
 ("Council of Nicaea", 325, None, "Rejects Arianism; the first form of the Nicene Creed.", False),
 ("Athanasius lists the 27 New Testament books", 367, None, "His Easter letter is the earliest list matching today's New Testament.", False),
 ("Council of Constantinople", 381, None, "Affirms the deity of the Holy Spirit; the Nicene Creed takes its familiar form.", False),
 ("Jerome's Vulgate", 382, 405, "The Latin Bible used in the West for a thousand years.", True),
 ("Augustine of Hippo", 354, 430, "Confessions, City of God; deeply shapes Western theology.", False),
 ("Council of Chalcedon", 451, None, "Christ confessed as one person in two natures.", False),
 ("Rule of St Benedict", 530, None, "Foundation of Western monasticism.", True),
 ("East–West Schism", 1054, None, "Mutual excommunications divide Rome and Constantinople.", False),
 ("The Crusades", 1095, 1291, "Military campaigns to the Holy Land.", False),
 ("Thomas Aquinas, Summa Theologiae", 1265, 1274, "Synthesis of Aristotle and Christian theology.", False),
 ("Wycliffe Bible", 1382, None, "First complete English Bible, translated from the Vulgate.", True),
 ("Jan Hus burned", 1415, None, "Czech reformer condemned at the Council of Constance.", False),
 ("Luther's Ninety-five Theses", 1517, None, "The start of the Protestant Reformation.", False),
 ("Schleitheim Confession", 1527, None, "Early Anabaptist statement of faith.", False),
 ("Augsburg Confession", 1530, None, "The chief Lutheran confession.", False),
 ("Act of Supremacy", 1534, None, "Henry VIII becomes head of the Church of England.", False),
 ("Calvin's Institutes", 1536, None, "First edition of the Reformed classic.", False),
 ("Council of Trent", 1545, 1563, "The Catholic response to the Reformation.", False),
 ("Heidelberg Catechism", 1563, None, "A widely used Reformed catechism.", False),
 ("First Baptist congregation", 1609, None, "John Smyth's English exiles in Amsterdam.", False),
 ("King James Version", 1611, None, "The Authorized Version of the English Bible.", False),
 ("Synod of Dort", 1618, 1619, "Reformed response to the Remonstrants.", False),
 ("Westminster Confession", 1646, None, "Confession of English and Scottish Presbyterianism.", False),
 ("First Great Awakening", 1730, 1745, "Revivals under Edwards, Whitefield and the Wesleys.", True),
 ("Wesley's Aldersgate experience", 1738, None, "A turning point in the beginnings of Methodism.", False),
 ("Azusa Street revival", 1906, None, "Los Angeles revival at the start of modern Pentecostalism.", False),
 ("Second Vatican Council", 1962, 1965, "Reforms in Catholic worship and relations with other churches.", False)]:
    e(H, t, s, en, summary=sm, approx=ap)

lanes = [
 ("bible", "Biblical history", "Traditional chronology (early Exodus, from 1 Kings 6:1). Many scholars prefer later dates for the patriarchs and the Exodus."),
 ("united", "United kingdom", "Approximate; Saul's reign length is uncertain (Acts 13:21)."),
 ("israel", "Kings of Israel", "Reign dates after Edwin R. Thiele, The Mysterious Numbers of the Hebrew Kings. Other systems differ by a few years; overlaps are co-regencies."),
 ("judah", "Kings of Judah", "Reign dates after Edwin R. Thiele. Overlaps are co-regencies."),
 ("prophets", "Prophets", "Approximate periods of ministry from the books' headings."),
 ("powers", "Who was in power", "Standard ancient Near Eastern chronology (middle chronology for Mesopotamia)."),
 ("rulers", "Rulers", "Reign dates as usually given in histories of the ancient world."),
 ("world", "World and culture", "Approximate where marked."),
 ("church", "Church history", "Dates as commonly given in histories of the church.")]

books = {
 "Gen": [[12, -2091, -1805]], "Exod": [[1, -1526, -1445]], "Lev": [[1, -1445, -1445]], "Num": [[1, -1445, -1406]], "Deut": [[1, -1406, -1406]],
 "Josh": [[1, -1406, -1375]], "Judg": [[1, -1375, -1050]], "Ruth": [[1, -1150, -1100]], "1Sam": [[1, -1105, -1010]], "2Sam": [[1, -1010, -970]],
 "1Kgs": [[1, -970, -931], [12, -931, -853]], "2Kgs": [[1, -853, -722], [18, -715, -586], [25, -586, -561]],
 "1Chr": [[10, -1010, -970]], "2Chr": [[1, -970, -931], [10, -931, -586], [36, -586, -538]],
 "Ezra": [[1, -538, -516], [7, -458, -458]], "Neh": [[1, -445, -432]], "Esth": [[1, -483, -473]],
 "Prov": [[1, -950, -700]], "Eccl": [[1, -935, -935]], "Song": [[1, -960, -960]],
 "Isa": [[1, -740, -681]], "Jer": [[1, -627, -586]], "Lam": [[1, -586, -586]], "Ezek": [[1, -593, -571]], "Dan": [[1, -605, -536]],
 "Hos": [[1, -755, -715]], "Amos": [[1, -760, -750]], "Jonah": [[1, -785, -775]], "Mic": [[1, -735, -700]], "Nah": [[1, -660, -630]],
 "Hab": [[1, -609, -598]], "Zeph": [[1, -640, -621]], "Hag": [[1, -520, -520]], "Zech": [[1, -520, -518]], "Mal": [[1, -440, -420]],
 "1Macc": [[1, -175, -134]], "2Macc": [[1, -180, -161]],
 "Matt": [[1, -5, -4], [3, 27, 30]], "Mark": [[1, 27, 30]], "Luke": [[1, -6, -4], [3, 27, 30]], "John": [[1, 27, 30]],
 "Acts": [[1, 30, 35], [13, 46, 57], [21, 57, 62]],
 "Rom": [[1, 57, 57]], "1Cor": [[1, 55, 55]], "2Cor": [[1, 56, 56]], "Gal": [[1, 49, 49]], "Eph": [[1, 60, 62]], "Phil": [[1, 60, 62]],
 "Col": [[1, 60, 62]], "1Thess": [[1, 51, 51]], "2Thess": [[1, 51, 52]], "1Tim": [[1, 62, 64]], "2Tim": [[1, 67, 67]], "Titus": [[1, 62, 64]],
 "Phlm": [[1, 60, 62]], "Heb": [[1, 65, 69]], "Jas": [[1, 45, 49]], "1Pet": [[1, 62, 64]], "2Pet": [[1, 65, 67]], "1John": [[1, 85, 95]],
 "2John": [[1, 85, 95]], "3John": [[1, 85, 95]], "Jude": [[1, 65, 80]], "Rev": [[1, 95, 95]]}

json.dump(dict(eras=[dict(title=t, start=s, end=en) for t, s, en in eras],
               lanes=[dict(id=i, title=t, source=s) for i, t, s in lanes],
               events=L, books=books), open("sys.argv[1]", "w"), ensure_ascii=False, indent=0)
print(len(L), "events")
