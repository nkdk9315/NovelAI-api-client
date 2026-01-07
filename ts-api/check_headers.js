import axios from 'axios';

async function checkHeaders(url) {
    try {
        const response = await axios.head(url);
        console.log('Headers for:', url);
        console.log(JSON.stringify(response.headers, null, 2));
    } catch (error) {
        console.error('Error fetching headers:', error.message);
    }
}

const url = "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true";
checkHeaders(url);
