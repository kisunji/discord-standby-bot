//! Multilingual translations for "One more" phrase.

use rand::Rng;

/// Returns a random translation of "One more" from 100 different languages.
/// Returns (translation, language_name).
pub fn get_random_one_more() -> (&'static str, &'static str) {
    let translations = [
        // Indo-European Languages
        ("One more", "English"),
        ("Uno más", "Spanish"),
        ("Un de plus", "French"),
        ("Noch einer", "German"),
        ("Еще один", "Russian"),
        ("Ancora uno", "Italian"),
        ("Mais um", "Portuguese"),
        ("Jeszcze jeden", "Polish"),
        ("Încă unul", "Romanian"),
        ("Ще один", "Ukrainian"),
        ("Ještě jeden", "Czech"),
        ("Ešte jeden", "Slovak"),
        ("Još jedan", "Croatian/Serbian/Bosnian"),
        ("Още един", "Bulgarian"),
        ("Ένας ακόμα", "Greek"),
        ("Een meer", "Dutch"),
        ("En till", "Swedish"),
        ("En til", "Norwegian"),
        ("Én mere", "Danish"),
        ("Yksi lisää", "Finnish"),
        ("Még egy", "Hungarian"),
        ("Vienas daugiau", "Lithuanian"),
        ("Vēl viens", "Latvian"),
        ("Üks veel", "Estonian"),
        ("Eitt til", "Icelandic"),
        ("یکی دیگر", "Persian"),
        ("एक और", "Hindi"),
        ("আরও একটি", "Bengali"),
        ("ایک اور", "Urdu"),
        ("ਇੱਕ ਹੋਰ", "Punjabi"),
        ("એક વધુ", "Gujarati"),
        ("ಇನ್ನೂ ಒಂದು", "Kannada"),
        ("ఇంకా ఒకటి", "Telugu"),
        ("இன்னும் ஒன்று", "Tamil"),
        ("ഒന്നുകൂടി", "Malayalam"),
        ("තවත් එකක්", "Sinhala"),

        // Sino-Tibetan Languages
        ("再来一个", "Chinese Simplified"),
        ("再來一個", "Chinese Traditional"),
        ("もう一つ", "Japanese"),
        ("하나 더", "Korean"),
        ("နောက်တစ်ခု", "Burmese"),
        ("ཡང་གཅིག", "Tibetan"),

        // Semitic Languages
        ("עוד אחד", "Hebrew"),
        ("واحد آخر", "Arabic"),
        ("ሌላ አንድ", "Amharic"),

        // Turkic Languages
        ("Bir tane daha", "Turkish"),
        ("Тағы бір", "Kazakh"),
        ("Yana bitta", "Uzbek"),
        ("Дагы бир", "Kyrgyz"),
        ("Yana bir", "Azerbaijani"),
        ("Тагы бер", "Tatar"),

        // Niger-Congo Languages
        ("Ọkan sii", "Yoruba"),
        ("Otu ọzọ", "Igbo"),
        ("Chimodzi china", "Chichewa"),
        ("Bumwe", "Shona"),
        ("Moja zaidi", "Swahili"),
        ("E nngwe", "Tswana"),
        ("Eyinye", "Xhosa"),
        ("Enye eyengeziwe", "Zulu"),

        // Austronesian Languages
        ("Isa pa", "Tagalog/Filipino"),
        ("Satu lagi", "Indonesian/Malay"),
        ("Isa pa", "Cebuano"),
        ("Tasi tano", "Chamorro"),
        ("Kotahi anō", "Maori"),
        ("E tasi atu", "Samoan"),
        ("E taha tale", "Fijian"),
        ("Hoʻokahi hou", "Hawaiian"),
        ("Še jeden", "Slovenian"),

        // Dravidian Languages
        ("ಇನ್ನೊಂದು", "Kannada"),
        ("மேலும் ஒன்று", "Tamil"),

        // Constructed Languages
        ("Unu pli", "Esperanto"),
        ("Unu pluse", "Interlingua"),

        // Austroasiatic Languages
        ("មួយទៀត", "Khmer"),
        ("ອີກອັນນຶ່ງ", "Lao"),
        ("Thêm một", "Vietnamese"),

        // Tai-Kadai Languages
        ("อีกหนึ่ง", "Thai"),

        // Japanese-Ryukyuan
        ("ちゅーてぃー", "Okinawan"),

        // Korean
        ("한 개 더", "Korean"),

        // Mongolic Languages
        ("Дахиад нэг", "Mongolian"),

        // Uralic Languages (more)
        ("Üks veel", "Estonian"),
        ("Vēl viens", "Latvian"),

        // Celtic Languages
        ("Un arall", "Welsh"),
        ("Aon eile", "Irish"),
        ("Aon eile", "Scottish Gaelic"),
        ("Unnane", "Manx"),

        // Baltic Languages
        ("Dar vienas", "Lithuanian"),

        // Albanian
        ("Edhe një", "Albanian"),

        // Armenian
        ("ԵՒս մեկ", "Armenian"),

        // Georgian
        ("კიდევ ერთი", "Georgian"),

        // Basque
        ("Bat gehiago", "Basque"),

        // Maltese
        ("Wieħed ieħor", "Maltese"),

        // Afrikaans
        ("Nog een", "Afrikaans"),

        // Additional variants and regional dialects
        ("Wan moa", "Jamaican Patois"),
        ("Un altro", "Corsican"),
        ("Unu más", "Galician"),
        ("Bat gehiago", "Basque"),
        ("Un autre", "French Canadian"),
        ("Unu di più", "Sicilian")
    ];
    
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..translations.len());
    translations[index]
}
