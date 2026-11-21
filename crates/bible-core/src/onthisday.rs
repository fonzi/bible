//! Today in church history: dated events across the whole church, East and
//! West, Catholic, Orthodox and Protestant. Compiled for this app from
//! standard church histories; dates are Gregorian (Julian for events before 1582).

use chrono::{Datelike, NaiveDate};

pub struct Event {
    pub month: u32,
    pub day: u32,
    pub year: i32,
    pub text: &'static str,
}

const E: &[(u32, u32, i32, &str)] = &[
    (1, 3, 1521, "Pope Leo X excommunicates Martin Luther in the bull Decet Romanum Pontificem."),
    (1, 6, 1412, "Traditional birth date of Joan of Arc."),
    (1, 8, 1956, "Jim Elliot and four other missionaries are killed in Ecuador; their families later return to serve the Waorani."),
    (1, 14, 1604, "The Hampton Court Conference ends; James I approves a new translation that becomes the King James Version."),
    (1, 21, 1525, "The first adult baptisms in Zurich mark the beginning of the Anabaptist movement."),
    (1, 25, 1077, "Emperor Henry IV begins three days of penance in the snow at Canossa before Pope Gregory VII."),
    (1, 28, 1547, "Henry VIII dies; Edward VI's reign brings Protestant reform to England."),
    (2, 11, 1858, "Bernadette Soubirous reports her first vision at Lourdes."),
    (2, 14, 869, "Cyril, apostle to the Slavs with his brother Methodius, dies in Rome."),
    (3, 28, 1515, "Teresa of Ávila, Carmelite reformer and mystic, is born."),
    (4, 8, 1546, "The Council of Trent decrees the canon of Scripture, deuterocanonical books included."),
    (4, 29, 1380, "Catherine of Siena dies in Rome."),
    (5, 13, 1917, "The first of the reported apparitions at Fátima, Portugal."),
    (7, 21, 1773, "Pope Clement XIV suppresses the Society of Jesus."),
    (8, 6, 1221, "Dominic, founder of the Order of Preachers, dies in Bologna."),
    (8, 20, 1153, "Bernard of Clairvaux dies."),
    (9, 4, 1965, "Albert Schweitzer, theologian and missionary doctor, dies at Lambaréné."),
    (9, 12, 1683, "The relief of Vienna; in thanksgiving the feast of the Holy Name of Mary, kept this day, is extended to the whole Church."),
    (9, 25, 1555, "The Peace of Augsburg lets each prince choose Lutheran or Catholic faith for his lands."),
    (10, 16, 1555, "Hugh Latimer and Nicholas Ridley are burned at Oxford."),
    (11, 9, 324, "Traditional date of the dedication of the Lateran Basilica, the pope’s cathedral in Rome."),
    (12, 21, 1597, "Peter Canisius, Jesuit catechist of the Catholic Reformation, dies at Fribourg."),
    (12, 14, 1591, "John of the Cross, poet and Carmelite reformer, dies."),
    (2, 4, 1555, "John Rogers, editor of the Matthew Bible, is the first Protestant burned under Mary I."),
    (2, 8, 1587, "Mary, Queen of Scots, is executed."),
    (2, 18, 1546, "Martin Luther dies in Eisleben, the town of his birth."),
    (2, 23, 155, "Traditional date of the martyrdom of Polycarp of Smyrna."),
    (2, 24, 1527, "The Schleitheim Confession of the Swiss Brethren is adopted."),
    (2, 27, 1594, "Henry IV is crowned king of France after converting to Catholicism."),
    (3, 2, 1791, "John Wesley dies in London."),
    (3, 7, 1274, "Thomas Aquinas dies on his way to the Second Council of Lyon."),
    (3, 12, 604, "Pope Gregory the Great dies in Rome."),
    (3, 17, 461, "Traditional date of the death of Patrick, missionary to Ireland."),
    (3, 21, 1556, "Thomas Cranmer, author of the Book of Common Prayer, is burned at Oxford."),
    (3, 31, 1492, "The Alhambra Decree orders the expulsion of Jews from Spain."),
    (4, 9, 1945, "Dietrich Bonhoeffer is executed at Flossenbürg concentration camp."),
    (4, 13, 1598, "The Edict of Nantes grants French Protestants religious freedom."),
    (4, 18, 1521, "Luther refuses to recant before the Diet of Worms."),
    (4, 21, 1109, "Anselm of Canterbury dies."),
    (4, 30, 311, "Emperor Galerius issues an edict of toleration ending the Great Persecution in the East."),
    (5, 4, 1521, "Luther is taken into hiding at the Wartburg, where he translates the New Testament into German."),
    (5, 8, 1373, "Julian of Norwich receives the visions of her Revelations of Divine Love."),
    (5, 20, 325, "The Council of Nicaea opens (traditional date)."),
    (5, 24, 1738, "John Wesley's heart is \"strangely warmed\" at Aldersgate Street in London."),
    (5, 29, 1453, "Constantinople falls to the Ottoman Turks."),
    (5, 30, 1431, "Joan of Arc is burned at Rouen."),
    (6, 1, 1660, "Mary Dyer, a Quaker, is hanged on Boston Common."),
    (6, 9, 597, "Columba, missionary to Scotland, dies on Iona."),
    (6, 13, 1525, "Martin Luther marries Katharina von Bora."),
    (6, 15, 1215, "King John seals Magna Carta, whose first clause guarantees the freedom of the English church."),
    (6, 22, 431, "The Council of Ephesus opens; it affirms Mary as Theotokos."),
    (6, 25, 1530, "The Augsburg Confession is read before Emperor Charles V."),
    (7, 1, 1523, "Hendrik Voes and Johann Esch, the first Protestant martyrs, are burned in Brussels."),
    (7, 6, 1415, "Jan Hus is burned at the Council of Constance."),
    (7, 8, 1741, "Jonathan Edwards preaches \"Sinners in the Hands of an Angry God\" at Enfield."),
    (7, 15, 1099, "Crusaders capture Jerusalem in the First Crusade."),
    (7, 16, 1054, "Cardinal Humbert lays a bull of excommunication on the altar of Hagia Sophia; the East–West Schism."),
    (7, 25, 1471, "Thomas à Kempis, author of The Imitation of Christ, dies."),
    (7, 31, 1556, "Ignatius of Loyola, founder of the Jesuits, dies in Rome."),
    (8, 13, 1727, "A renewal of the Moravian Church at Herrnhut begins a century-long prayer vigil."),
    (8, 24, 1572, "The St Bartholomew's Day massacre of Huguenots begins in Paris."),
    (8, 28, 430, "Augustine of Hippo dies as the Vandals besiege his city."),
    (9, 3, 590, "Gregory the Great is consecrated Pope."),
    (9, 13, 1541, "John Calvin returns to Geneva for good."),
    (9, 16, 1620, "The Mayflower sails from Plymouth with Separatist Pilgrims."),
    (9, 21, 1522, "Luther's German New Testament (the \"September Testament\") is published."),
    (9, 30, 420, "Jerome, translator of the Vulgate, dies in Bethlehem."),
    (10, 4, 1226, "Feast of Francis of Assisi, who died at the Portiuncula on the evening before, in 1226."),
    (10, 6, 1536, "William Tyndale is strangled and burned at Vilvoorde, praying \"Lord, open the King of England's eyes.\""),
    (10, 11, 1962, "The Second Vatican Council opens in Rome."),
    (10, 22, 1685, "Louis XIV revokes the Edict of Nantes."),
    (10, 28, 312, "Constantine defeats Maxentius at the Milvian Bridge."),
    (10, 31, 1517, "Martin Luther posts the Ninety-five Theses in Wittenberg."),
    (10, 31, 1999, "Lutherans and Catholics sign the Joint Declaration on the Doctrine of Justification."),
    (11, 1, 1950, "Pope Pius XII defines the dogma of the Assumption of Mary."),
    (11, 10, 1483, "Martin Luther is born in Eisleben."),
    (11, 11, 397, "Martinmas: Martin of Tours, who died three days earlier, is buried at Tours."),
    (11, 13, 354, "Augustine of Hippo is born in Thagaste, North Africa."),
    (11, 17, 1558, "Mary I dies; Elizabeth I's reign brings the Elizabethan Settlement."),
    (11, 18, 1302, "Pope Boniface VIII issues the bull Unam Sanctam."),
    (11, 24, 1572, "John Knox, leader of the Scottish Reformation, dies in Edinburgh."),
    (11, 27, 1095, "Pope Urban II preaches the First Crusade at Clermont."),
    (12, 3, 1552, "Francis Xavier, missionary to India and Japan, dies near China."),
    (12, 4, 1563, "The Council of Trent closes."),
    (12, 6, 343, "Traditional date of the death of Nicholas of Myra."),
    (12, 10, 1520, "Luther burns the papal bull Exsurge Domine in Wittenberg."),
    (12, 13, 1545, "The Council of Trent opens."),
    (12, 25, 800, "Charlemagne is crowned emperor by Pope Leo III in Rome."),
    (12, 24, 1223, "Francis of Assisi sets up a living nativity scene at Greccio."),
    (12, 29, 1170, "Thomas Becket is murdered in Canterbury Cathedral."),
];

pub fn events() -> impl Iterator<Item = Event> {
    E.iter().map(|(m, d, y, t)| Event {
        month: *m,
        day: *d,
        year: *y,
        text: t,
    })
}

/// Events on this calendar day; if none, the next one within a week (with its date).
pub fn for_date(date: NaiveDate) -> (bool, Vec<Event>) {
    let today: Vec<Event> = events()
        .filter(|e| e.month == date.month() && e.day == date.day())
        .collect();
    if !today.is_empty() {
        return (true, today);
    }
    for k in 1..=7 {
        let d = date + chrono::Duration::days(k);
        let soon: Vec<Event> = events()
            .filter(|e| e.month == d.month() && e.day == d.day())
            .collect();
        if !soon.is_empty() {
            return (false, soon);
        }
    }
    (false, vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_valid() {
        for e in events() {
            assert!(
                NaiveDate::from_ymd_opt(2024, e.month, e.day).is_some(),
                "{}",
                e.text
            );
        }
        let (exact, ev) = for_date(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
        assert!(exact && ev.len() == 2);
        let (exact, ev) = for_date(NaiveDate::from_ymd_opt(2026, 11, 18).unwrap());
        assert!(exact && ev[0].year == 1302);
    }
}
