const axios = require('axios');

async function checkHeaders(url) {
    try {
        const response = await axios.head(url);
        console.log('Headers for:', url);
        console.log(JSON.stringify(response.headers, null, 2));
    } catch (error) {
        if (error.response) {
            console.log('Status:', error.response.status);
            console.log('Headers:', JSON.stringify(error.response.headers, null, 2));
        } else {
            console.error('Error fetching headers:', error.message);
        }
    }
}

const url = "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true";
checkHeaders(url);
