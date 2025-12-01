const API_BASE = "http://127.0.0.2:8080";

let lastHeight = null;

async function fetchLatestBlock() {
  try {
    const res = await fetch(API_BASE + "/latest_block");
    const data = await res.json();

    if (!data.block) return;

    const block = data.block;

    // если новый хайт — обновляем
    if (lastHeight === null || block.header.height !== lastHeight) {
      lastHeight = block.header.height;
      document.getElementById("latestBlock").textContent =
        JSON.stringify(block, null, 2);
    }
  } catch (e) {
    console.error("fetchLatestBlock error", e);
  }
}

// опрос каждые 2 секунды
setInterval(fetchLatestBlock, 2000);
fetchLatestBlock(); // сразу первый раз

// поиск по block_id
document.getElementById("findBlockBtn").onclick = async () => {
  const id = document.getElementById("blockIdInput").value.trim();
  if (!id) return;

  const res = await fetch(API_BASE + "/block/" + encodeURIComponent(id));
  const data = await res.json();
  document.getElementById("searchResult").textContent =
    JSON.stringify(data.block, null, 2);
};

// поиск по tx_hash
document.getElementById("findTxBtn").onclick = async () => {
  const h = document.getElementById("txHashInput").value.trim();
  if (!h) return;

  const res = await fetch(API_BASE + "/tx/" + encodeURIComponent(h));
  const data = await res.json();

  if (!data.block) {
    document.getElementById("searchResult").textContent = "TX not found";
    return;
  }

  // блок с одной транзой
  const block = data.block;
  const tx = block.body.txs && block.body.txs.length > 0 ? block.body.txs[0] : null;

  document.getElementById("searchResult").textContent =
    JSON.stringify(
      {
        block_id: block.block_id,
        height: block.header.height,
        tx, // сама транзакция
      },
      null,
      2
    );
};
