# Kurulum

> Henüz yayımlanmış bir sürüm yoktur. İlk sürüm yayımlandığında aşağıdaki adımlar geçerli olacaktır. O zamana kadar uygulamayı [kaynak koddan derleyebilirsiniz](DERLEME.md).

## Gereksinimler

- Windows 10 (1803+) veya Windows 11, 64 bit
- Microsoft Edge WebView2 Çalışma Zamanı — Windows 11'de hazır gelir. Yoksa yükleyici küçük bir önyükleyici indirerek sessizce kurar (internet bağlantısı gerekir).

Rust, Node.js veya başka bir geliştirme aracı **gerekmez**.

## Kurulum adımları

1. [Sürümler](https://github.com/efecrtgl/ivily-optimizer/releases) sayfasından `Ivily Optimizer_x.y.z_x64-setup.exe` dosyasını indirin.
2. İsterseniz dosyanın SHA-256 özetini sürümdeki `SHA256SUMS.txt` ile karşılaştırın:
   ```powershell
   Get-FileHash ".\Ivily Optimizer_0.1.0_x64-setup.exe" -Algorithm SHA256
   ```
3. Yükleyiciyi çalıştırın. Kurulum **yönetici hakkı gerektirmez** ve yalnızca kullanıcı hesabınıza yapılır:
   - Uygulama: `%LOCALAPPDATA%\Ivily Optimizer\`
   - Başlat menüsü kısayolu: "Ivily Optimizer"

### Windows SmartScreen uyarısı

Yükleyici şu anda bir Authenticode kod imzalama sertifikasıyla imzalanmamıştır. Bu nedenle Windows SmartScreen "Windows bilgisayarınızı korudu" uyarısı gösterebilir. Dosyayı yalnızca resmi GitHub sürüm sayfasından indirdiyseniz "Ek bilgi > Yine de çalıştır" seçeneğiyle devam edebilirsiniz. (Uygulama içi güncellemeler ise ayrıca minisign imzasıyla doğrulanır; bkz. [GUNCELLEME.md](GUNCELLEME.md).)

## Güncelleme

Uygulama yeni sürümleri kendisi denetler (Ayarlar > Güncellemeler). Hakkında sayfasından "İndir ve kur" ile güncelleyebilirsiniz.

## Kaldırma

**Ayarlar > Uygulamalar > Yüklü uygulamalar > Ivily Optimizer > Kaldır** veya Başlat menüsündeki kaldırma kısayolunu kullanın.

Kaldırma uygulamayı ve kısayolları siler. Ayarlarınız, geçmişiniz ve **karantinadaki dosyalarınız** `%LOCALAPPDATA%\IvilyOptimizer` klasöründe kalır; böylece yanlışlıkla kaldırma veri kaybına yol açmaz. Bunları da silmek isterseniz:

1. Önce uygulamada Karantina sayfasını gözden geçirin ve gerekli dosyaları geri yükleyin.
2. Ardından `%LOCALAPPDATA%\IvilyOptimizer` klasörünü silin.

"Windows ile başlat" ayarı açıksa kaldırmadan önce kapatmanız önerilir; aksi halde zararsız ama geçersiz bir başlangıç kaydı kalabilir.
