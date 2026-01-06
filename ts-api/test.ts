import { NovelAIClient } from './src/client';
import dotenv from 'dotenv';
dotenv.config();

const client = new NovelAIClient();
// カラー化
const colorizeResult = (async() => {
  const result = await client.augmentImage({
    req_type: "emotion",
    prompt: "sad",
    defry: 0,
    image: "./reference/139533151_p3_master1200.jpg",
    save_dir: "./output/augment/"
  });
  console.log(result);
})();
// 表情変換
// const emotionResult = await client.augmentImage({
//   req_type: "emotion",
//   image: "./face_image.png",
//   width: 832,
//   height: 1216,
//   prompt: "happy;;",  // ;;が必要
//   defry: 0,
//   save_dir: "./output",
// });
// アップスケール
// const upscaleResult = await client.upscaleImage({
//   image: "./small_image.png",
//   width: 512,
//   height: 768,
//   scale: 4,
//   save_dir: "./output",
// });