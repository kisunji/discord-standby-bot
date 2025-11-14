//! Multilingual translations for "One more" phrase.

use rand::Rng;

/// Returns a random translation of "One more" from 100 different languages.
pub fn get_random_one_more() -> &'static str {
    let translations = [
        // Indo-European Languages
        "One more",              // English
        "Uno más",              // Spanish
        "Un de plus",           // French
        "Noch einer",           // German
        "Еще один",             // Russian (Yeshchyo odin)
        "Ancora uno",           // Italian
        "Mais um",              // Portuguese
        "Jeszcze jeden",        // Polish
        "Încă unul",            // Romanian
        "Ще один",              // Ukrainian (Shche odyn)
        "Ještě jeden",          // Czech
        "Ešte jeden",           // Slovak
        "Još jedan",            // Croatian/Serbian/Bosnian
        "Още един",             // Bulgarian (Oshte edin)
        "Ένας ακόμα",           // Greek (Énas akóma)
        "Een meer",             // Dutch
        "En till",              // Swedish
        "En til",               // Norwegian
        "Én mere",              // Danish
        "Yksi lisää",           // Finnish
        "Még egy",              // Hungarian
        "Vienas daugiau",       // Lithuanian
        "Vēl viens",            // Latvian
        "Üks veel",             // Estonian
        "Eitt til",             // Icelandic
        "یکی دیگر",             // Persian (Yeki dīgar)
        "एक और",                // Hindi (Ek aur)
        "আরও একটি",            // Bengali (Aro ekti)
        "ایک اور",              // Urdu (Aik aur)
        "ਇੱਕ ਹੋਰ",             // Punjabi (Ikk hor)
        "એક વધુ",               // Gujarati (Ek vadhu)
        "ಇನ್ನೂ ಒಂದು",          // Kannada (Innū ondu)
        "ఇంకా ఒకటి",           // Telugu (Iṅkā okaṭi)
        "இன்னும் ஒன்று",       // Tamil (Iṉṉum oṉṟu)
        "ഒന്നുകൂടി",            // Malayalam (Onnukūṭi)
        "තවත් එකක්",           // Sinhala (Tavat ekak)
        
        // Sino-Tibetan Languages
        "再来一个",              // Chinese Simplified (Zài lái yīgè)
        "再來一個",              // Chinese Traditional (Zài lái yīgè)
        "もう一つ",              // Japanese (Mō hitotsu)
        "하나 더",               // Korean (Hana deo)
        "နောက်တစ်ခု",          // Burmese (Naukhtakkhu)
        "ཡང་གཅིག",             // Tibetan (Yang gcig)
        
        // Semitic Languages
        "עוד אחד",              // Hebrew (Od echad)
        "واحد آخر",             // Arabic (Wāḥid ākhar)
        "ሌላ አንድ",              // Amharic (Lela andi)
        
        // Turkic Languages
        "Bir tane daha",        // Turkish
        "Тағы бір",             // Kazakh (Tağı bir)
        "Yana bitta",           // Uzbek
        "Дагы бир",             // Kyrgyz (Dağı bir)
        "Yana bir",             // Azerbaijani
        "Тагы бер",             // Tatar (Tağı ber)
        
        // Niger-Congo Languages
        "Ọkan sii",             // Yoruba
        "Otu ọzọ",              // Igbo
        "Chimodzi china",       // Chichewa
        "Bumwe",                // Shona
        "Moja zaidi",           // Swahili
        "E nngwe",              // Tswana
        "Eyinye",               // Xhosa
        "Enye eyengeziwe",      // Zulu
        
        // Austronesian Languages
        "Isa pa",               // Tagalog/Filipino
        "Satu lagi",            // Indonesian/Malay
        "Isa pa",               // Cebuano
        "Tasi tano",            // Chamorro
        "Kotahi anō",           // Maori
        "E tasi atu",           // Samoan
        "E taha tale",          // Fijian
        "Hoʻokahi hou",         // Hawaiian
        "Še jeden",             // Slovenian
        
        // Dravidian Languages
        "ಇನ್ನೊಂದು",            // Kannada (Iṉṉoṃdu)
        "மேலும் ஒன்று",         // Tamil (Mēlum oṉṟu)
        
        // Constructed Languages
        "Unu pli",              // Esperanto
        "Unu pluse",            // Interlingua
        
        // Austroasiatic Languages
        "មួយទៀត",               // Khmer (Muəy tiət)
        "ອີກອັນນຶ່ງ",           // Lao (Īk ʼan nưng)
        "Thêm một",             // Vietnamese
        
        // Tai-Kadai Languages
        "อีกหนึ่ง",             // Thai (Īk h̄nưng)
        
        // Japanese-Ryukyuan
        "ちゅーてぃー",           // Okinawan (Chūtī)
        
        // Korean
        "한 개 더",              // Korean (Han gae deo)
        
        // Mongolic Languages
        "Дахиад нэг",           // Mongolian (Dakhiad neg)
        
        // Uralic Languages (more)
        "Üks veel",             // Estonian
        "Vēl viens",            // Latvian (repeat - different dialect)
        
        // Celtic Languages
        "Un arall",             // Welsh
        "Aon eile",             // Irish
        "Aon eile",             // Scottish Gaelic
        "Unnane",               // Manx
        
        // Baltic Languages
        "Dar vienas",           // Lithuanian (alt)
        
        // Albanian
        "Edhe një",             // Albanian
        
        // Armenian
        "ԵՒս մեկ",               // Armenian (Yev's mek)
        
        // Georgian
        "კიდევ ერთი",           // Georgian (K'idev erti)
        
        // Basque
        "Bat gehiago",          // Basque
        
        // Maltese
        "Wieħed ieħor",         // Maltese
        
        // Afrikaans
        "Nog een",              // Afrikaans
        
        // Additional variants and regional dialects
        "Wan moa",              // Jamaican Patois
        "Un altro",             // Corsican
        "Unu más",              // Galician
        "Bat gehiago",          // Basque (repeat different)
        "Un autre",             // French Canadian
        "Unu di più",           // Sicilian
    ];
    
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..translations.len());
    translations[index]
}
