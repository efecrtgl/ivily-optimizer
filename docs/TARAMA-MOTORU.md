# Tarama motoru

Kaynak: `src-tauri/src/scan/`.

## Gezgin (`walker.rs`)

Tüm taramalar (disk analizi, temizlik, eski dosyalar, yinelenen dosyalar) aynı gezgini kullanır.

- **Paralellik:** Her tarama, ayarlardaki "tarama yoğunluğu" kadar iş parçacığına sahip ayrı bir `rayon` havuzu kurar (varsayılan 2). Disk çekişmesini sınırlamak için varsayılan değer muhafazakârdır.
- **İptal:** Her klasörde iptal bayrağı denetlenir; iptal edilen tarama "kısmi" olarak işaretlenir.
- **Bağlantı noktaları:** `FILE_ATTRIBUTE_REPARSE_POINT` taşıyan klasörler (junction, sembolik bağlantı, bağlama noktası) **izlenmez**. Böylece sonsuz döngüler, aynı verinin iki kez sayılması ve hedef dışına taşma önlenir. Atlanan bağlantılar sayılır ve raporlanır. Bu davranış, kendi köküne işaret eden bir junction ile test edilir.
- **Hatalar:** Erişilemeyen klasörler taramayı durdurmaz; neden (erişim reddedildi, kilitli, bulunamadı) ile birlikte ilk 500 hata saklanır, toplam sayı her zaman doğrudur.
- **Uzun yollar:** Rust standart kütüphanesi `MAX_PATH` sınırını aşan yollara `\\?\` önekini otomatik ekler.
- **Hariç tutmalar:** Ayarlardaki yollar (alt klasörleriyle birlikte) ve uzantılar her taramada uygulanır.
- **Meta veri:** Windows'ta `DirEntry::metadata()` bağlantıları izlemez ve `FindNextFile` verisinden geldiği için dosya başına ek sistem çağrısı gerektirmez.
- **Bulut dosyaları:** OneDrive yer tutucuları (`RECALL_ON_DATA_ACCESS` vb.) okunmaz; boyutları ayrıca raporlanır, temizlik ve yinelenen analizine dahil edilmez.

## İlerleme

Kesin ilerleme her zaman bilinemez. Kural:

- Bir birim kökü (ör. `C:\`) taranıyorsa ilerleme, **kullanılan alana göre tahmin** olarak gösterilir, `%99` ile sınırlanır ve "yaklaşık" etiketi taşır.
- Aşamalı işlerde (yinelenen dosya doğrulaması, temizlik) gerçek `yapılan / toplam` değeri gösterilir.
- Diğer durumlarda belirsiz (indeterminate) ilerleme çubuğu ve gerçek sayaçlar (dosya, klasör, bayt, hata, süre) gösterilir. İlerleme hiçbir zaman uydurulmaz.

## Disk analizi (`tree.rs`)

Her klasör için: doğrudan dosya sayısı/boyutu, alt ağaç toplamları, küçük dosya sayısı, en yeni değişiklik, en derin alt düzey ve 8 kovalı boyut histogramı. Toplamlar tarama sonunda derinliğe göre aşağıdan yukarıya hesaplanır. Ayrıca uzantı istatistikleri ve en büyük 1.000 dosya (sınırlı yığın) tutulur.

### Küçük dosya yoğunluğu

Bir klasör, eşiğin (varsayılan 16 KB) altındaki dosya sayısı ayarlanan en azı (varsayılan 2.000) aşıyorsa adaydır. Küçük dosyalarının %80'inden fazlası tek bir alt klasördeyse asıl yoğunluk o alt klasörde olduğundan üst klasör ayrıca listelenmez. Olası etki (Yüksek/Orta/Düşük) yalnızca sayı ve orana dayanır; bu bir performans sorunu kanıtı değildir ve arayüzde bu sınırlama açıkça yazılır. `node_modules`, Rust `target`, `.git`, Python ortamları, paket önbellekleri gibi bilinen klasörler için muhafazakâr öneriler verilir.

## Eski dosyalar

Dosyalar **son değiştirilme** zamanına göre seçilir. Son erişim zamanı Windows'ta varsayılan olarak güvenilir şekilde güncellenmediği için kullanılmaz. Tarihi okunamayan dosyalar dahil edilmez ve sayıları raporlanır.

## Yinelenen dosyalar (`dupes.rs`)

1. Boyuta göre gruplama (tekil boyutlar elenir).
2. İlk ve son 64 KB'ın BLAKE3 özeti ile ön eleme.
3. Tüm içeriğin BLAKE3 özeti ile kesin doğrulama (iptal edilebilir, bayt bazında ilerleme).
4. Birim seri numarası + dosya dizini ile **sabit bağlantıların** (hard link) ayıklanması: aynı fiziksel dosyanın iki adını silmek yer kazandırmaz.

Önerilen seçim her gruptan en eski oluşturulan kopyayı korur ve korunan konumlardaki dosyaları asla seçmez. Arayüz, bir grubun tüm kopyalarının seçilmesine izin vermez.
